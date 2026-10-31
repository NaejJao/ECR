//! Basic cash-entry and ledger domain types for the ECR project.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EntryKind {
    #[default]
    Inflow,
    Outflow,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CashEntry {
    pub kind: EntryKind,
    pub amount_minor: u64,
    pub description: String,
}

impl CashEntry {
    pub fn new(
        kind: EntryKind,
        amount_minor: u64,
        description: impl Into<String>,
    ) -> Result<Self, &'static str> {
        if amount_minor == 0 {
            return Err("amount must be greater than zero");
        }
        let description = description.into();
        if description.trim().is_empty() {
            return Err("description must not be empty");
        }
        Ok(Self {
            kind,
            amount_minor,
            description,
        })
    }
}

#[derive(Debug, Default)]
pub struct Ledger {
    entries: Vec<CashEntry>,
}

impl Ledger {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn record(&mut self, entry: CashEntry) {
        self.entries.push(entry);
    }
    pub fn entries(&self) -> &[CashEntry] {
        &self.entries
    }

    pub fn totals(&self) -> Result<LedgerTotals, &'static str> {
        let mut totals = LedgerTotals::default();
        for entry in &self.entries {
            let total = match entry.kind {
                EntryKind::Inflow => &mut totals.inflows_minor,
                EntryKind::Outflow => &mut totals.outflows_minor,
            };
            *total = total
                .checked_add(entry.amount_minor)
                .ok_or("ledger total overflow")?;
        }
        Ok(totals)
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct LedgerTotals {
    pub inflows_minor: u64,
    pub outflows_minor: u64,
}

impl LedgerTotals {
    pub fn balance_minor(self) -> Option<i64> {
        i64::try_from(self.inflows_minor)
            .ok()?
            .checked_sub(i64::try_from(self.outflows_minor).ok()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn totals_track_inflows_and_outflows() {
        let mut ledger = Ledger::new();
        ledger.record(CashEntry::new(EntryKind::Inflow, 2500, "sale").unwrap());
        ledger.record(CashEntry::new(EntryKind::Outflow, 700, "supplies").unwrap());
        let totals = ledger.totals().unwrap();
        assert_eq!(totals.inflows_minor, 2500);
        assert_eq!(totals.outflows_minor, 700);
        assert_eq!(totals.balance_minor(), Some(1800));
    }

    #[test]
    fn entry_rejects_zero_amount_and_blank_description() {
        assert!(CashEntry::new(EntryKind::Inflow, 0, "sale").is_err());
        assert!(CashEntry::new(EntryKind::Inflow, 100, "  ").is_err());
    }
}
