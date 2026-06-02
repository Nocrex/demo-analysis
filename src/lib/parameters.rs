use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub enum Parameter {
    Float(f32),
    Int(i32),
    Bool(bool),
}

#[derive(Debug)]
pub enum ParameterError {
    TypeMismatch,
}

impl TryFrom<&Parameter> for f32 {
    type Error = ParameterError;

    fn try_from(param: &Parameter) -> Result<Self, Self::Error> {
        if let Parameter::Float(f) = param {
            Ok(*f)
        } else {
            Err(ParameterError::TypeMismatch)
        }
    }
}

impl TryFrom<&Parameter> for i32 {
    type Error = ParameterError;

    fn try_from(param: &Parameter) -> Result<Self, Self::Error> {
        if let Parameter::Int(i) = param {
            Ok(*i)
        } else {
            Err(ParameterError::TypeMismatch)
        }
    }
}

impl TryFrom<&Parameter> for bool {
    type Error = ParameterError;

    fn try_from(param: &Parameter) -> Result<Self, Self::Error> {
        if let Parameter::Bool(b) = param {
            Ok(*b)
        } else {
            Err(ParameterError::TypeMismatch)
        }
    }
}

impl TryFrom<&Parameter> for Parameter {
    type Error = ParameterError;

    fn try_from(param: &Parameter) -> Result<Self, Self::Error> {
        Ok(param.clone())
    }
}

impl Clone for Parameter {
    fn clone(&self) -> Self {
        match self {
            Parameter::Float(f) => Parameter::Float(*f),
            Parameter::Int(i) => Parameter::Int(*i),
            Parameter::Bool(b) => Parameter::Bool(*b),
        }
    }
}

impl Serialize for Parameter {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Parameter::Float(f) => serializer.serialize_f32(*f),
            Parameter::Int(i) => serializer.serialize_i32(*i),
            Parameter::Bool(b) => serializer.serialize_bool(*b),
        }
    }
}

impl<'a> Deserialize<'a> for Parameter {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'a>,
    {
        struct ParameterVisitor;

        impl<'de> serde::de::Visitor<'de> for ParameterVisitor {
            type Value = Parameter;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("an f32, i32, or bool")
            }

            fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Parameter::Float(value as f32))
            }

            fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Parameter::Int(value as i32))
            }

            fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Parameter::Int(value as i32))
            }

            fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Parameter::Bool(value))
            }
        }

        deserializer.deserialize_any(ParameterVisitor)
    }
}

// Maps parameter names to their values.
pub type Parameters = HashMap<String, Parameter>;

// Maps algorithm names to their parameters.
pub type Config = HashMap<String, Parameters>;

pub fn get_parameter_value<T>(params: &Parameters, param_name: &str) -> T
where
    T: for<'a> TryFrom<&'a Parameter, Error = ParameterError>,
{
    match params.get(param_name) {
        Some(param) => match T::try_from(param) {
            Ok(value) => value,
            Err(_) => panic!("Parameter {} has wrong type", param_name),
        },
        None => panic!("Parameter {} not found", param_name),
    }
}