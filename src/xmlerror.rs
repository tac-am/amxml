//
// xmlerror.rs
//
// amxml: XML processor with XPath.
// Copyright (C) 2018 KOYAMA Hiro <tac@amris.co.jp>
//

//!
//! XmlError: Error type and description.
//!

use std::error::Error;
use std::fmt;

// =====================================================================
//
#[derive(Debug, PartialEq)]
/// Type code of XmlError.
///
pub enum XmlErrorType {
    CantOccur,
    Unimplemented,
    XmlSyntaxError,
    XPathSyntaxError,
    StaticError,
    DynamicError,
    TypeError,
}

const ERROR_PREFIX: [(XmlErrorType, &str); 7] = [
    ( XmlErrorType::CantOccur, "Can't Occur: problem in amxml library:" ),
    ( XmlErrorType::Unimplemented, "Feature not inplemented yet:" ),
    ( XmlErrorType::XmlSyntaxError, "Syntax Error in XML:" ),
    ( XmlErrorType::XPathSyntaxError, "Syntax Error in XPath:" ),
    ( XmlErrorType::StaticError, "Static Error:" ),
    ( XmlErrorType::DynamicError, "Dynamic Error:" ),
    ( XmlErrorType::TypeError, "Type Error:" ),
];

#[derive(Debug)]
pub struct XmlError {
    error_type: XmlErrorType,
    descri: String,
}

impl fmt::Display for XmlError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.description())
    }
}

impl Error for XmlError {
    fn description(&self) -> &str {
        return self.descri.as_str();
    }
}

// =====================================================================
//
pub fn xmlerror(error_type: XmlErrorType, descri: &str) -> Box<XmlError> {

    let mut prefix = "Unknown error:";
    for el in ERROR_PREFIX.iter() {
        if el.0 == error_type {
            prefix = el.1;
            break;
        }
    }

    return Box::new(XmlError {
        error_type: error_type,
        descri: format!("{} {}", prefix, descri),
    });
}

// ---------------------------------------------------------------------
//
macro_rules! cant_occur {
    (
        $( $e:expr ),*
    ) => {
        xmlerror(XmlErrorType::CantOccur, &format!( $($e),+ ))
    }
}

#[allow(unused_macros)]
macro_rules! uninplemented {
    (
        $( $e:expr ),*
    ) => {
        xmlerror(XmlErrorType::Unimplemented, &format!( $($e),+ ))
    }
}

macro_rules! xml_syntax_error {
    (
        $( $e:expr ),*
    ) => {
        xmlerror(XmlErrorType::XmlSyntaxError, &format!( $($e),+ ))
    }
}

macro_rules! xpath_syntax_error {
    (
        $( $e:expr ),*
    ) => {
        xmlerror(XmlErrorType::XPathSyntaxError, &format!( $($e),+ ))
    }
}

#[allow(unused_macros)]
macro_rules! static_error {
    (
        $( $e:expr ),*
    ) => {
        xmlerror(XmlErrorType::StaticError, &format!( $($e),+ ))
    }
}

#[allow(unused_macros)]
macro_rules! dynamic_error {
    (
        $( $e:expr ),*
    ) => {
        xmlerror(XmlErrorType::DynamicError, &format!( $($e),+ ))
    }
}

#[allow(unused_macros)]
macro_rules! type_error {
    (
        $( $e:expr ),*
    ) => {
        xmlerror(XmlErrorType::TypeError, &format!( $($e),+ ))
    }
}

