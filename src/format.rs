use std::fmt::Write;
use upon::{fmt, Value};
use convert_case::{Case, Casing};

pub fn lower(f: &mut fmt::Formatter, value: &Value) -> fmt::Result {
    match value {
        Value::None => Err(fmt::Error::from("unable to format None"))?,
        Value::String(s) => write!(f, "{}", s.to_case(Case::Lower))?,
        _ => Err(fmt::Error::from("expected to format a string"))?,
    }
    Ok(())
}

pub fn upper(f: &mut fmt::Formatter, value: &Value) -> fmt::Result {
    match value {
        Value::None => Err(fmt::Error::from("unable to format None"))?,
        Value::String(s) => write!(f, "{}", s.to_case(Case::Upper))?,
        _ => Err(fmt::Error::from("expected to format a string"))?,
    }
    Ok(())
}

pub fn snake(f: &mut fmt::Formatter, value: &Value) -> fmt::Result {
    match value {
        Value::None => Err(fmt::Error::from("unable to format None"))?,
        Value::String(s) => write!(f, "{}", s.to_case(Case::Snake))?,
        _ => Err(fmt::Error::from("expected to format a string"))?,
    }
    Ok(())
}

pub fn kebab(f: &mut fmt::Formatter, value: &Value) -> fmt::Result {
    match value {
        Value::None => Err(fmt::Error::from("unable to format None"))?,
        Value::String(s) => write!(f, "{}", s.to_case(Case::Kebab))?,
        _ => Err(fmt::Error::from("expected to format a string"))?,
    }
    Ok(())
}

pub fn pascal(f: &mut fmt::Formatter, value: &Value) -> fmt::Result {
    match value {
        Value::None => Err(fmt::Error::from("unable to format None"))?,
        Value::String(s) => write!(f, "{}", s.to_case(Case::Pascal))?,
        _ => Err(fmt::Error::from("expected to format a string"))?,
    }
    Ok(())
}

pub fn camel(f: &mut fmt::Formatter, value: &Value) -> fmt::Result {
    match value {
        Value::None => Err(fmt::Error::from("unable to format None"))?,
        Value::String(s) => write!(f, "{}", s.to_case(Case::Camel))?,
        _ => Err(fmt::Error::from("expected to format a string"))?,
    }
    Ok(())
}

pub fn title(f: &mut fmt::Formatter, value: &Value) -> fmt::Result {
    match value {
        Value::None => Err(fmt::Error::from("unable to format None"))?,
        Value::String(s) => write!(f, "{}", s.to_case(Case::Title))?,
        _ => Err(fmt::Error::from("expected to format a string"))?,
    }
    Ok(())
}
