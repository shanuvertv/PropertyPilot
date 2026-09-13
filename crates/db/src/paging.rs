//! Shared list parameters: paging, whitelisted sorting, free-text search.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortDir {
    Asc,
    Desc,
}

impl SortDir {
    pub fn parse(s: Option<&str>) -> SortDir {
        match s.map(|v| v.to_ascii_lowercase()) {
            Some(v) if v == "desc" => SortDir::Desc,
            _ => SortDir::Asc,
        }
    }
    fn sql(self) -> &'static str {
        match self {
            SortDir::Asc => "ASC",
            SortDir::Desc => "DESC",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ListQuery {
    pub q: Option<String>,
    pub page: i64,
    pub page_size: i64,
    pub sort: Option<String>,
    pub dir: SortDir,
}

impl ListQuery {
    pub const MAX_PAGE_SIZE: i64 = 200;

    pub fn new(
        q: Option<String>,
        page: Option<i64>,
        page_size: Option<i64>,
        sort: Option<String>,
        dir: Option<&str>,
    ) -> Self {
        let q = q.map(|s| s.trim().to_owned()).filter(|s| !s.is_empty());
        ListQuery {
            q,
            page: page.unwrap_or(1).max(1),
            page_size: page_size.unwrap_or(25).clamp(1, Self::MAX_PAGE_SIZE),
            sort,
            dir: SortDir::parse(dir),
        }
    }

    /// Every row, for reports and exports (still capped so a runaway query cannot exhaust memory).
    pub fn all(sort: Option<&str>, dir: Option<&str>) -> Self {
        ListQuery {
            q: None,
            page: 1,
            page_size: 50_000,
            sort: sort.map(str::to_owned),
            dir: SortDir::parse(dir),
        }
    }

    pub fn offset(&self) -> i64 {
        (self.page - 1) * self.page_size
    }

    /// `ILIKE` pattern for the search term, or `None` when there is no term.
    pub fn like(&self) -> Option<String> {
        self.q
            .as_ref()
            .map(|q| format!("%{}%", q.replace('%', "\\%").replace('_', "\\_")))
    }

    /// Resolve the requested sort key against a whitelist of `(key, sql expression)` pairs.
    /// Unknown keys fall back to the first entry, so user input never reaches the SQL text.
    pub fn order_by(&self, whitelist: &[(&str, &str)]) -> String {
        let (_, expr) = self
            .sort
            .as_deref()
            .and_then(|k| whitelist.iter().find(|(key, _)| *key == k))
            .copied()
            .unwrap_or(whitelist[0]);
        // Apply the direction to every column of a multi-column expression
        // ("b.name, u.unit_number" must become "b.name DESC, u.unit_number DESC").
        let dir = self.dir.sql();
        let cols = expr
            .split(',')
            .map(|c| format!("{} {} NULLS LAST", c.trim(), dir))
            .collect::<Vec<_>>()
            .join(", ");
        format!(" ORDER BY {cols} ")
    }
}

#[derive(Debug, Clone)]
pub struct PageResult<T> {
    pub items: Vec<T>,
    pub total: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamps_paging_and_whitelists_sort() {
        let q = ListQuery::new(
            Some("  ab%  ".into()),
            Some(0),
            Some(9999),
            Some("evil; DROP".into()),
            Some("desc"),
        );
        assert_eq!(q.page, 1);
        assert_eq!(q.page_size, ListQuery::MAX_PAGE_SIZE);
        assert_eq!(q.like().unwrap(), "%ab\\%%");
        assert_eq!(
            q.order_by(&[("name", "b.name"), ("code", "b.code")]),
            " ORDER BY b.name DESC NULLS LAST "
        );
        let q2 = ListQuery::new(None, Some(3), Some(10), Some("code".into()), None);
        assert_eq!(q2.offset(), 20);
        assert_eq!(
            q2.order_by(&[("name", "b.name"), ("code", "b.code")]),
            " ORDER BY b.code ASC NULLS LAST "
        );
        // every column of a compound sort follows the direction
        let q3 = ListQuery::new(None, None, None, Some("unit".into()), Some("desc"));
        assert_eq!(
            q3.order_by(&[("unit", "b.name, u.unit_number")]),
            " ORDER BY b.name DESC NULLS LAST, u.unit_number DESC NULLS LAST "
        );
        assert!(q2.like().is_none());
    }
}
