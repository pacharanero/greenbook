use crate::error::ParseError;
use chrono::{Days, Months, NaiveDate};
use serde::{Deserialize, Deserializer};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AgeOffset {
    pub years: u32,
    pub months: u32,
    pub weeks: u32,
    pub days: u32,
}

impl AgeOffset {
    pub fn to_date(&self, dob: NaiveDate) -> NaiveDate {
        let with_months = dob
            .checked_add_months(Months::new(self.years * 12 + self.months))
            .expect("date overflow adding months to DOB");
        with_months
            .checked_add_days(Days::new((self.weeks * 7 + self.days) as u64))
            .expect("date overflow adding days to DOB")
    }
}

impl fmt::Display for AgeOffset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut parts: Vec<String> = Vec::new();
        if self.years > 0 {
            parts.push(format!("{} year{}", self.years, plural(self.years)));
        }
        if self.months > 0 {
            parts.push(format!("{} month{}", self.months, plural(self.months)));
        }
        if self.weeks > 0 {
            parts.push(format!("{} week{}", self.weeks, plural(self.weeks)));
        }
        if self.days > 0 {
            parts.push(format!("{} day{}", self.days, plural(self.days)));
        }
        if parts.is_empty() {
            write!(f, "0 days")
        } else {
            write!(f, "{}", parts.join(" "))
        }
    }
}

fn plural(n: u32) -> &'static str {
    if n == 1 {
        ""
    } else {
        "s"
    }
}

impl FromStr for AgeOffset {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let tokens: Vec<&str> = s.split_whitespace().collect();
        if tokens.is_empty() {
            return Err(ParseError::Empty);
        }
        let mut offset = AgeOffset::default();
        let mut i = 0;
        while i < tokens.len() {
            let n: u32 = tokens[i]
                .parse()
                .map_err(|_| ParseError::InvalidNumber(tokens[i].to_string()))?;
            let unit = tokens.get(i + 1).ok_or(ParseError::MissingUnit(n))?;
            match unit.to_ascii_lowercase().as_str() {
                "year" | "years" => offset.years = n,
                "month" | "months" => offset.months = n,
                "week" | "weeks" => offset.weeks = n,
                "day" | "days" => offset.days = n,
                other => return Err(ParseError::UnknownUnit(other.to_string())),
            }
            i += 2;
        }
        Ok(offset)
    }
}

impl<'de> Deserialize<'de> for AgeOffset {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_units() {
        assert_eq!("8 weeks".parse::<AgeOffset>().unwrap().weeks, 8);
        assert_eq!("12 months".parse::<AgeOffset>().unwrap().months, 12);
        assert_eq!("3 years".parse::<AgeOffset>().unwrap().years, 3);
    }

    #[test]
    fn parses_compound() {
        let a: AgeOffset = "3 years 4 months".parse().unwrap();
        assert_eq!(a.years, 3);
        assert_eq!(a.months, 4);

        let b: AgeOffset = "14 weeks 6 days".parse().unwrap();
        assert_eq!(b.weeks, 14);
        assert_eq!(b.days, 6);
    }

    #[test]
    fn to_date_uses_calendar_arithmetic() {
        let dob = NaiveDate::from_ymd_opt(2025, 10, 29).unwrap();
        let age: AgeOffset = "12 months".parse().unwrap();
        assert_eq!(
            age.to_date(dob),
            NaiveDate::from_ymd_opt(2026, 10, 29).unwrap()
        );

        let eight_weeks: AgeOffset = "8 weeks".parse().unwrap();
        assert_eq!(
            eight_weeks.to_date(dob),
            NaiveDate::from_ymd_opt(2025, 12, 24).unwrap()
        );
    }
}
