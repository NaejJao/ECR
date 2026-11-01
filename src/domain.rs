//! Business rules and formatting shared by the user interface and application state.
//!
//! This module contains no screen layout. It owns price parsing, category
//! reconciliation, sale counting, and the rules for associating toppings.

use crate::model::{Item, Receipt};
use std::{collections::BTreeMap, path::PathBuf};

/// Attaches a topping to the most recently added base item in the given category.
///
/// Returns `Some(true)` when newly attached, `Some(false)` when that same topping
/// is already on the base item, and `None` when no matching base item exists.
/// A repeated click is deliberately a no-op: toppings are unique per base item.
pub(crate) fn attach_topping(order: &mut [Item], topping: Item, category: usize) -> Option<bool> {
    let parent = order
        .iter_mut()
        .rev()
        .find(|item| item.category_index == category)?;
    if parent
        .toppings
        .iter()
        .any(|existing| existing.name == topping.name)
    {
        return Some(false);
    }
    parent.toppings.push(topping);
    Some(true)
}

/// Parses a non-negative price entered with either a decimal comma or decimal point.
///
/// Returns the value in centimes. Inputs with more than two fractional digits,
/// both decimal separators, or negative values are rejected; zero is allowed for free items.
pub(crate) fn parse_price(s: &str) -> Option<u64> {
    let s = s.trim();
    if s.is_empty() || s.starts_with('-') || (s.contains(',') && s.contains('.')) {
        return None;
    }
    let normalized = s.replace(',', ".");
    let mut p = normalized.split('.');
    let whole = p.next()?.parse::<u64>().ok()?;
    let frac = p.next().unwrap_or("0");
    if p.next().is_some() || frac.len() > 2 || !frac.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let c = match frac.len() {
        0 => 0,
        1 => frac.parse::<u64>().ok()? * 10,
        2 => frac.parse::<u64>().ok()?,
        _ => return None,
    };
    whole.checked_mul(100)?.checked_add(c)
}

/// Formats an amount stored in centimes using French decimal and currency notation.
pub(crate) fn money(c: u64) -> String {
    format!("{},{:02} €", c / 100, c % 100)
}

/// Resolves the platform-specific application data directory and appends `file`.
///
/// Uses `%APPDATA%` on Windows, Application Support on macOS, and XDG data
/// storage (or `~/.local/share`) on Linux and other Unix-like systems.
pub(crate) fn data_path(file: &str) -> Result<PathBuf, String> {
    #[cfg(target_os = "windows")]
    let base = std::env::var_os("APPDATA").map(PathBuf::from);
    #[cfg(target_os = "macos")]
    let base =
        std::env::var_os("HOME").map(|x| PathBuf::from(x).join("Library/Application Support"));
    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|x| PathBuf::from(x).join(".local/share")));
    base.map(|p| p.join("ecr").join(file))
        .ok_or_else(|| "Impossible de trouver le dossier de données de l’application".into())
}

/// Counts sold base items by category, excluding their nested toppings.
///
/// Current menu categories are initialized to zero so unsold categories remain
/// visible in the daily report. Legacy category names are reconciled when possible.
pub(crate) fn main_item_counts(orders: &[Receipt], categories: &[String]) -> BTreeMap<String, u64> {
    let mut counts = BTreeMap::new();
    for category in categories {
        if !counts
            .keys()
            .any(|known: &String| known.eq_ignore_ascii_case(category.trim()))
        {
            counts.insert(category.clone(), 0);
        }
    }
    for item in orders.iter().flat_map(|order| &order.items) {
        let recorded = if item.category.trim().is_empty() {
            "Non catégorisé"
        } else {
            item.category.trim()
        };
        let category = canonical_category(recorded, categories);
        let count = counts.entry(category).or_insert(0_u64);
        *count = count.saturating_add(u64::from(item.quantity));
    }
    counts
}

/// Orders category totals by the menu's configured order, then appends legacy names.
pub(crate) fn ordered_category_counts(
    counts: &BTreeMap<String, u64>,
    categories: &[String],
) -> Vec<(String, u64)> {
    let mut ordered = Vec::new();
    for category in categories {
        if let Some((name, count)) = counts
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case(category.trim()))
        {
            if !ordered
                .iter()
                .any(|(known, _): &(String, u64)| known.eq_ignore_ascii_case(name))
            {
                ordered.push((name.clone(), *count));
            }
        }
    }
    for (name, count) in counts {
        if !ordered
            .iter()
            .any(|(known, _)| known.eq_ignore_ascii_case(name))
        {
            ordered.push((name.clone(), *count));
        }
    }
    ordered
}

/// Aggregates base products sold in one category as `(quantity, subtotal)` pairs.
///
/// Toppings are intentionally omitted from the breakdown; their amounts are
/// already included in each completed receipt's total.
pub(crate) fn main_products_in_category(
    orders: &[Receipt],
    selected_category: &str,
    categories: &[String],
) -> BTreeMap<String, (u64, u64)> {
    let mut details = BTreeMap::new();
    for item in orders.iter().flat_map(|order| &order.items) {
        let recorded = if item.category.trim().is_empty() {
            "Non catégorisé"
        } else {
            item.category.trim()
        };
        if canonical_category(recorded, categories) != selected_category {
            continue;
        }
        let row = details.entry(item.name.clone()).or_insert((0_u64, 0_u64));
        row.0 = row.0.saturating_add(u64::from(item.quantity));
        row.1 = row
            .1
            .saturating_add(item.price.saturating_mul(u64::from(item.quantity)));
    }
    details
}

/// Maps a recorded category name to its current menu name when unambiguous.
///
/// Matching first ignores letter case. A single one-edit spelling difference is
/// accepted only for category names of at least six characters; ambiguous matches
/// and larger differences are preserved as separate historical categories.
pub(crate) fn canonical_category(recorded: &str, categories: &[String]) -> String {
    if let Some(exact) = categories
        .iter()
        .find(|category| category.trim().eq_ignore_ascii_case(recorded))
    {
        return exact.trim().to_owned();
    }
    let close_matches: Vec<&String> = categories
        .iter()
        .filter(|category| {
            category.trim().chars().count() >= 6 && one_edit_apart(category.trim(), recorded)
        })
        .collect();
    if close_matches.len() == 1 {
        close_matches[0].trim().to_owned()
    } else {
        recorded.to_owned()
    }
}

/// Returns whether two strings differ by at most one insertion, deletion, or replacement.
pub(crate) fn one_edit_apart(left: &str, right: &str) -> bool {
    let left: Vec<char> = left.to_lowercase().chars().collect();
    let right: Vec<char> = right.to_lowercase().chars().collect();
    if left.len().abs_diff(right.len()) > 1 {
        return false;
    }
    let (mut i, mut j, mut edits) = (0, 0, 0);
    while i < left.len() && j < right.len() {
        if left[i] == right[j] {
            i += 1;
            j += 1;
            continue;
        }
        edits += 1;
        if edits > 1 {
            return false;
        }
        if left.len() > right.len() {
            i += 1;
        } else if right.len() > left.len() {
            j += 1;
        } else {
            i += 1;
            j += 1;
        }
    }
    if i < left.len() || j < right.len() {
        edits += 1;
    }
    edits <= 1
}

/// Returns the stable RGB palette entry for a category position.
pub(crate) fn theme(i: usize) -> [u8; 3] {
    [
        [210, 232, 211],
        [211, 225, 245],
        [249, 225, 190],
        [237, 215, 231],
        [211, 234, 231],
        [242, 220, 205],
        [228, 224, 246],
        [231, 232, 204],
    ][i % 8]
}

/// Provides the default pale-green theme for older records without stored colors.
pub(crate) fn default_theme() -> [u8; 3] {
    theme(0)
}
