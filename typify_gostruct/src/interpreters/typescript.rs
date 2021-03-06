use crate::ast::{DataType, Field, FieldType, StructDeclaration, TagKey, AST};

use super::{Interpreter, InterpreterError};

pub struct TypeScriptInterpreter();

static OPENING_BRACKET: char = '{';

static CLOSING_BRACKET: char = '}';

impl Interpreter for TypeScriptInterpreter {
    fn interpret(&self, ast: Vec<crate::ast::AST>) -> Result<String, InterpreterError> {
        let mut result = String::new();
        for item in ast {
            let struct_results = match item {
                AST::Declaration(declaration) => self.interpret_struct(*declaration),
                _ => return Err(InterpreterError::ExpectedStructFoundField),
            };
            result.push_str(&struct_results)
        }
        Ok(result)
    }
}

impl TypeScriptInterpreter {
    pub fn new() -> Self {
        Self {}
    }
    fn get_field_type(&self, data_type: DataType) -> super::FieldType {
        match data_type {
            DataType::Number => super::FieldType::Normal("number".to_string()),
            DataType::String => super::FieldType::Normal("string".to_string()),
            DataType::Boolean => super::FieldType::Normal("boolean".to_string()),
            DataType::Custom(custom) => super::FieldType::Normal(custom),
            DataType::Embedded => super::FieldType::Embedded,
        }
    }
    fn interpret_struct(&self, declaration: StructDeclaration) -> String {
        let mut result = format!("\n export interface {} = ", declaration.name);
        result.push(OPENING_BRACKET);

        for item in declaration.body {
            let field_result = self.interpret_field(item);
            result.push_str(&field_result)
        }
        result.push(CLOSING_BRACKET);
        result
    }

    fn interpret_field(&self, field: crate::ast::Field) -> String {
        let mut result = String::new();
        let field_result = match field {
            Field::Blank => String::new(),
            Field::Plain(field_name, field_type) => {
                let field_type = self.convert_field_type(field_type);
                match field_type {
                    super::FieldType::Normal(field_type) => {
                        format!("{} : {},", field_name.0, field_type)
                    }
                    super::FieldType::Embedded => format!("...{}, ", field_name.0),
                }
            }
            Field::WithTags(field_name, field_type, field_tags) => {
                self.interpret_field_with_tags(field_name, field_type, field_tags)
            }
        };

        result.push_str(&field_result);
        result
    }

    fn convert_field_type(&self, field_type: FieldType) -> super::FieldType {
        let single_or_list_item_notation = match field_type {
            FieldType::List(_) => "[]",
            FieldType::One(_) => "",
        };
        let field_type = match field_type {
            FieldType::One(data_type) => self.get_field_type(data_type),
            FieldType::List(data_type) => self.get_field_type(data_type),
        };

        match field_type {
            super::FieldType::Normal(specified_type) => super::FieldType::Normal(format!(
                "{}{}",
                specified_type, single_or_list_item_notation
            )),
            super::FieldType::Embedded => super::FieldType::Embedded,
        }
    }

    fn interpret_field_with_tags(
        &self,
        field_name: crate::ast::FieldName,
        field_type: FieldType,
        tags: std::collections::HashMap<crate::ast::TagKey, crate::ast::TagValue>,
    ) -> String {
        let mut field_name = field_name.0;
        let field_type = self.convert_field_type(field_type);

        for (key, value) in &tags {
            if *key == TagKey("json".to_string()) {
                field_name = value.0.clone()
            }
        }
        match field_type {
            super::FieldType::Normal(field_type) => format!("{} : {}, ", field_name, field_type),
            super::FieldType::Embedded => format!("...{}, ", field_name), // TODO: find out later if its possible to have embedded fields with with JSON tags
        }
    }
}
