use crate::ast::Type;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RomitType {
    Text,
    Int,
    Decimal,
    Bool,
    Money,
    Pct,
    Days,
    Minutes,
    Enum(String),
    Unknown,
}

pub fn resolve_type_name(name: &str, enums: &[String]) -> RomitType {
    match name {
        "text" => RomitType::Text,
        "int" => RomitType::Int,
        "decimal" => RomitType::Decimal,
        "bool" => RomitType::Bool,
        "money" => RomitType::Money,
        "pct" => RomitType::Pct,
        "days" => RomitType::Days,
        "minutes" => RomitType::Minutes,
        other => {
            if enums.iter().any(|e| e == other) {
                RomitType::Enum(other.to_string())
            } else {
                RomitType::Unknown
            }
        }
    }
}

pub fn type_from_ast(t: &Type, enums: &[String]) -> RomitType {
    match t {
        Type::Named(n) => resolve_type_name(n, enums),
        Type::Span(_) => RomitType::Unknown,
    }
}

pub fn types_compatible(a: &RomitType, b: &RomitType) -> bool {
    if a == b {
        return true;
    }
    matches!(
        (a, b),
        (RomitType::Int, RomitType::Money)
            | (RomitType::Money, RomitType::Int)
            | (RomitType::Int, RomitType::Days)
            | (RomitType::Days, RomitType::Int)
            | (RomitType::Int, RomitType::Minutes)
            | (RomitType::Minutes, RomitType::Int)
            | (RomitType::Int, RomitType::Pct)
            | (RomitType::Pct, RomitType::Int)
            | (RomitType::Decimal, RomitType::Money)
            | (RomitType::Money, RomitType::Decimal)
    )
}

pub fn is_numeric(t: &RomitType) -> bool {
    matches!(
        t,
        RomitType::Int
            | RomitType::Decimal
            | RomitType::Money
            | RomitType::Pct
            | RomitType::Days
            | RomitType::Minutes
    )
}
