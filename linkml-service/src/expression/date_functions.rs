//! Date and time manipulation functions for LinkML expressions
//!
//! This module provides date/time functions for working with temporal data.

use super::functions::{BuiltinFunction, FunctionError};
use chrono::{DateTime, Datelike, Duration, Local, NaiveDate, Timelike, Utc};
use serde_json::Value;

/// now() - Get current timestamp
pub struct NowFunction;

impl BuiltinFunction for NowFunction {
    fn name(&self) -> &str {
        "now"
    }
    
    fn validate_arity(&self, args: &[Value]) -> Result<(), FunctionError> {
        if !args.is_empty() {
            return Err(FunctionError::wrong_arity(self.name(), "0", args.len()));
        }
        Ok(())
    }
    
    fn call(&self, _args: Vec<Value>) -> Result<Value, FunctionError> {
        let now = Utc::now();
        Ok(Value::String(now.to_rfc3339()))
    }
}

/// today() - Get today's date
pub struct TodayFunction;

impl BuiltinFunction for TodayFunction {
    fn name(&self) -> &str {
        "today"
    }
    
    fn validate_arity(&self, args: &[Value]) -> Result<(), FunctionError> {
        if !args.is_empty() {
            return Err(FunctionError::wrong_arity(self.name(), "0", args.len()));
        }
        Ok(())
    }
    
    fn call(&self, _args: Vec<Value>) -> Result<Value, FunctionError> {
        let today = Local::now().date_naive();
        Ok(Value::String(today.to_string()))
    }
}

/// date_parse() - Parse date from string
pub struct DateParseFunction;

impl BuiltinFunction for DateParseFunction {
    fn name(&self) -> &str {
        "date_parse"
    }
    
    fn validate_arity(&self, args: &[Value]) -> Result<(), FunctionError> {
        if args.len() != 1 && args.len() != 2 {
            return Err(FunctionError::wrong_arity(self.name(), "1 or 2", args.len()));
        }
        Ok(())
    }
    
    fn call(&self, args: Vec<Value>) -> Result<Value, FunctionError> {
        let date_str = match &args[0] {
            Value::String(s) => s,
            _ => return Err(FunctionError::invalid_argument(
                self.name(),
                "first argument must be a string",
            )),
        };
        
        // Try to parse with different formats
        let parsed = if args.len() == 2 {
            // Custom format provided
            match &args[1] {
                Value::String(format) => {
                    NaiveDate::parse_from_str(date_str, format)
                        .map_err(|_| FunctionError::invalid_argument(
                            self.name(),
                            "failed to parse date with provided format",
                        ))?
                }
                _ => return Err(FunctionError::invalid_argument(
                    self.name(),
                    "second argument must be a format string",
                )),
            }
        } else {
            // Try common formats
            NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
                .or_else(|_| NaiveDate::parse_from_str(date_str, "%Y/%m/%d"))
                .or_else(|_| NaiveDate::parse_from_str(date_str, "%m/%d/%Y"))
                .or_else(|_| NaiveDate::parse_from_str(date_str, "%d/%m/%Y"))
                .map_err(|_| FunctionError::invalid_argument(
                    self.name(),
                    "failed to parse date - try providing a format string",
                ))?
        };
        
        Ok(Value::String(parsed.to_string()))
    }
}

/// date_format() - Format date to string
pub struct DateFormatFunction;

impl BuiltinFunction for DateFormatFunction {
    fn name(&self) -> &str {
        "date_format"
    }
    
    fn validate_arity(&self, args: &[Value]) -> Result<(), FunctionError> {
        if args.len() != 2 {
            return Err(FunctionError::wrong_arity(self.name(), "2", args.len()));
        }
        Ok(())
    }
    
    fn call(&self, args: Vec<Value>) -> Result<Value, FunctionError> {
        let date_str = match &args[0] {
            Value::String(s) => s,
            _ => return Err(FunctionError::invalid_argument(
                self.name(),
                "first argument must be a date string",
            )),
        };
        
        let format = match &args[1] {
            Value::String(s) => s,
            _ => return Err(FunctionError::invalid_argument(
                self.name(),
                "second argument must be a format string",
            )),
        };
        
        // Parse the date
        let date = if let Ok(dt) = DateTime::parse_from_rfc3339(date_str) {
            dt.format(format).to_string()
        } else if let Ok(d) = NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
            d.format(format).to_string()
        } else {
            return Err(FunctionError::invalid_argument(
                self.name(),
                "failed to parse date",
            ));
        };
        
        Ok(Value::String(date))
    }
}

/// date_add() - Add duration to date
pub struct DateAddFunction;

impl BuiltinFunction for DateAddFunction {
    fn name(&self) -> &str {
        "date_add"
    }
    
    fn validate_arity(&self, args: &[Value]) -> Result<(), FunctionError> {
        if args.len() != 3 {
            return Err(FunctionError::wrong_arity(self.name(), "3", args.len()));
        }
        Ok(())
    }
    
    fn call(&self, args: Vec<Value>) -> Result<Value, FunctionError> {
        let date_str = match &args[0] {
            Value::String(s) => s,
            _ => return Err(FunctionError::invalid_argument(
                self.name(),
                "first argument must be a date string",
            )),
        };
        
        let amount = match &args[1] {
            Value::Number(n) => n.as_i64().unwrap_or(0),
            _ => return Err(FunctionError::invalid_argument(
                self.name(),
                "second argument must be a number",
            )),
        };
        
        let unit = match &args[2] {
            Value::String(s) => s,
            _ => return Err(FunctionError::invalid_argument(
                self.name(),
                "third argument must be a unit string (days, weeks, months, years)",
            )),
        };
        
        // Parse the date
        let date = if let Ok(dt) = DateTime::parse_from_rfc3339(date_str) {
            let duration = match unit.as_str() {
                "days" => Duration::days(amount),
                "weeks" => Duration::weeks(amount),
                "hours" => Duration::hours(amount),
                "minutes" => Duration::minutes(amount),
                "seconds" => Duration::seconds(amount),
                _ => return Err(FunctionError::invalid_argument(
                    self.name(),
                    "unsupported unit - use: days, weeks, hours, minutes, seconds",
                )),
            };
            (dt + duration).to_rfc3339()
        } else if let Ok(d) = NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
            let result = match unit.as_str() {
                "days" => d + Duration::days(amount),
                "weeks" => d + Duration::weeks(amount),
                "months" => {
                    // Handle months specially
                    let mut year = d.year();
                    let mut month = d.month() as i32 + amount as i32;
                    while month > 12 {
                        month -= 12;
                        year += 1;
                    }
                    while month < 1 {
                        month += 12;
                        year -= 1;
                    }
                    NaiveDate::from_ymd_opt(year, month as u32, d.day())
                        .unwrap_or(d)
                }
                "years" => {
                    NaiveDate::from_ymd_opt(d.year() + amount as i32, d.month(), d.day())
                        .unwrap_or(d)
                }
                _ => return Err(FunctionError::invalid_argument(
                    self.name(),
                    "for date-only values, use: days, weeks, months, years",
                )),
            };
            result.to_string()
        } else {
            return Err(FunctionError::invalid_argument(
                self.name(),
                "failed to parse date",
            ));
        };
        
        Ok(Value::String(date))
    }
}

/// date_diff() - Calculate difference between dates
pub struct DateDiffFunction;

impl BuiltinFunction for DateDiffFunction {
    fn name(&self) -> &str {
        "date_diff"
    }
    
    fn validate_arity(&self, args: &[Value]) -> Result<(), FunctionError> {
        if args.len() != 3 {
            return Err(FunctionError::wrong_arity(self.name(), "3", args.len()));
        }
        Ok(())
    }
    
    fn call(&self, args: Vec<Value>) -> Result<Value, FunctionError> {
        let date1_str = match &args[0] {
            Value::String(s) => s,
            _ => return Err(FunctionError::invalid_argument(
                self.name(),
                "first argument must be a date string",
            )),
        };
        
        let date2_str = match &args[1] {
            Value::String(s) => s,
            _ => return Err(FunctionError::invalid_argument(
                self.name(),
                "second argument must be a date string",
            )),
        };
        
        let unit = match &args[2] {
            Value::String(s) => s,
            _ => return Err(FunctionError::invalid_argument(
                self.name(),
                "third argument must be a unit string",
            )),
        };
        
        // Parse dates
        let diff = if let (Ok(dt1), Ok(dt2)) = (
            DateTime::parse_from_rfc3339(date1_str),
            DateTime::parse_from_rfc3339(date2_str),
        ) {
            let duration = dt2.signed_duration_since(dt1);
            match unit.as_str() {
                "days" => duration.num_days(),
                "hours" => duration.num_hours(),
                "minutes" => duration.num_minutes(),
                "seconds" => duration.num_seconds(),
                _ => return Err(FunctionError::invalid_argument(
                    self.name(),
                    "unsupported unit - use: days, hours, minutes, seconds",
                )),
            }
        } else if let (Ok(d1), Ok(d2)) = (
            NaiveDate::parse_from_str(date1_str, "%Y-%m-%d"),
            NaiveDate::parse_from_str(date2_str, "%Y-%m-%d"),
        ) {
            let duration = d2.signed_duration_since(d1);
            match unit.as_str() {
                "days" => duration.num_days(),
                "weeks" => duration.num_weeks(),
                _ => return Err(FunctionError::invalid_argument(
                    self.name(),
                    "for date-only values, use: days or weeks",
                )),
            }
        } else {
            return Err(FunctionError::invalid_argument(
                self.name(),
                "failed to parse dates",
            ));
        };
        
        Ok(Value::Number(serde_json::Number::from(diff)))
    }
}

/// year() - Extract year from date
pub struct YearFunction;

impl BuiltinFunction for YearFunction {
    fn name(&self) -> &str {
        "year"
    }
    
    fn validate_arity(&self, args: &[Value]) -> Result<(), FunctionError> {
        if args.len() != 1 {
            return Err(FunctionError::wrong_arity(self.name(), "1", args.len()));
        }
        Ok(())
    }
    
    fn call(&self, args: Vec<Value>) -> Result<Value, FunctionError> {
        let date_str = match &args[0] {
            Value::String(s) => s,
            _ => return Err(FunctionError::invalid_argument(
                self.name(),
                "argument must be a date string",
            )),
        };
        
        let year = if let Ok(dt) = DateTime::parse_from_rfc3339(date_str) {
            dt.year()
        } else if let Ok(d) = NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
            d.year()
        } else {
            return Err(FunctionError::invalid_argument(
                self.name(),
                "failed to parse date",
            ));
        };
        
        Ok(Value::Number(serde_json::Number::from(year)))
    }
}

/// month() - Extract month from date
pub struct MonthFunction;

impl BuiltinFunction for MonthFunction {
    fn name(&self) -> &str {
        "month"
    }
    
    fn validate_arity(&self, args: &[Value]) -> Result<(), FunctionError> {
        if args.len() != 1 {
            return Err(FunctionError::wrong_arity(self.name(), "1", args.len()));
        }
        Ok(())
    }
    
    fn call(&self, args: Vec<Value>) -> Result<Value, FunctionError> {
        let date_str = match &args[0] {
            Value::String(s) => s,
            _ => return Err(FunctionError::invalid_argument(
                self.name(),
                "argument must be a date string",
            )),
        };
        
        let month = if let Ok(dt) = DateTime::parse_from_rfc3339(date_str) {
            dt.month()
        } else if let Ok(d) = NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
            d.month()
        } else {
            return Err(FunctionError::invalid_argument(
                self.name(),
                "failed to parse date",
            ));
        };
        
        Ok(Value::Number(serde_json::Number::from(month)))
    }
}

/// day() - Extract day from date
pub struct DayFunction;

impl BuiltinFunction for DayFunction {
    fn name(&self) -> &str {
        "day"
    }
    
    fn validate_arity(&self, args: &[Value]) -> Result<(), FunctionError> {
        if args.len() != 1 {
            return Err(FunctionError::wrong_arity(self.name(), "1", args.len()));
        }
        Ok(())
    }
    
    fn call(&self, args: Vec<Value>) -> Result<Value, FunctionError> {
        let date_str = match &args[0] {
            Value::String(s) => s,
            _ => return Err(FunctionError::invalid_argument(
                self.name(),
                "argument must be a date string",
            )),
        };
        
        let day = if let Ok(dt) = DateTime::parse_from_rfc3339(date_str) {
            dt.day()
        } else if let Ok(d) = NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
            d.day()
        } else {
            return Err(FunctionError::invalid_argument(
                self.name(),
                "failed to parse date",
            ));
        };
        
        Ok(Value::Number(serde_json::Number::from(day)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    
    #[test]
    fn test_now_today() {
        let now_fn = NowFunction;
        let result = now_fn.call(vec![]).expect("now() should succeed");
        assert!(matches!(result, Value::String(_)));
        
        let today_fn = TodayFunction;
        let result = today_fn.call(vec![]).expect("today() should succeed");
        assert!(matches!(result, Value::String(_)));
    }
    
    #[test]
    fn test_date_parse() {
        let parse = DateParseFunction;
        
        // Standard format
        let result = parse.call(vec![json!("2024-01-15")]).expect("should parse standard date format");
        assert_eq!(result, json!("2024-01-15"));
        
        // Custom format
        let result = parse.call(vec![json!("15/01/2024"), json!("%d/%m/%Y")]).expect("should parse custom date format");
        assert_eq!(result, json!("2024-01-15"));
    }
    
    #[test]
    fn test_date_format() {
        let format = DateFormatFunction;
        
        let result = format.call(vec![json!("2024-01-15"), json!("%Y/%m/%d")]).expect("should format date as Y/m/d");
        assert_eq!(result, json!("2024/01/15"));
        
        let result = format.call(vec![json!("2024-01-15"), json!("%B %d, %Y")]).expect("should format date with month name");
        assert_eq!(result, json!("January 15, 2024"));
    }
    
    #[test]
    fn test_date_add() {
        let add = DateAddFunction;
        
        // Add days
        let result = add.call(vec![json!("2024-01-15"), json!(10), json!("days")]).expect("should add days to date");
        assert_eq!(result, json!("2024-01-25"));
        
        // Add months
        let result = add.call(vec![json!("2024-01-15"), json!(2), json!("months")]).expect("should add months to date");
        assert_eq!(result, json!("2024-03-15"));
    }
    
    #[test]
    fn test_date_diff() {
        let diff = DateDiffFunction;
        
        let result = diff.call(vec![
            json!("2024-01-15"),
            json!("2024-01-25"),
            json!("days")
        ]).expect("should calculate date difference");
        assert_eq!(result, json!(10));
    }
    
    #[test]
    fn test_date_parts() {
        let year = YearFunction;
        let month = MonthFunction;
        let day = DayFunction;
        
        let date = json!("2024-03-15");
        
        assert_eq!(year.call(vec![date.clone()]).expect("should extract year"), json!(2024));
        assert_eq!(month.call(vec![date.clone()]).expect("should extract month"), json!(3));
        assert_eq!(day.call(vec![date.clone()]).expect("should extract day"), json!(15));
    }
}