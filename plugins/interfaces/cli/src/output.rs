use serde::Serialize;
use serde_json::to_string_pretty;

pub fn print_json<T: Serialize>(value: &T) -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", to_string_pretty(value)?);
    Ok(())
}

pub fn validate_pagination(
    page: usize,
    page_size: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    if page < 1 {
        return Err("页码必须大于 0".into());
    }
    if !(1..=100).contains(&page_size) {
        return Err("每页数量必须在 1 到 100 之间".into());
    }
    Ok(())
}

pub fn validate_enum(
    value: Option<&str>,
    allowed: &[&str],
    field: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(value) = value
        && !allowed.contains(&value)
    {
        return Err(format!("{field}不支持 {value}，可选值：{}", allowed.join(", ")).into());
    }
    Ok(())
}

pub fn require_confirm(confirm: bool, action: &str) -> Result<(), Box<dyn std::error::Error>> {
    if confirm {
        Ok(())
    } else {
        Err(format!("{action} 属于破坏性操作，请显式传入 --confirm").into())
    }
}
