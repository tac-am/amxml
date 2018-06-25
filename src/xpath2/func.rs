//
// xpath2/func.rs
//
// amxml: XML processor with XPath.
// Copyright (C) 2018 KOYAMA Hiro <tac@amris.co.jp>
//

use std::error::Error;
use std::f64;
use std::i64;
use std::usize;

use xmlerror::*;
use xpath2::eval::*;
use xpath2::xitem::*;
use xpath2::xsequence::*;

// ---------------------------------------------------------------------
//
fn usize_to_i64(n: usize) -> i64 {
    return n as i64;
}

fn f64_to_i64(f: f64) -> i64 {
    if f.is_infinite() {
        if f.is_sign_positive() {
            return i64::MAX;
        } else {
            return i64::MIN;
        }
    } else {
        return f as i64;
    }
}

// ---------------------------------------------------------------------
// 函数表。
// - 実行時に、函数の実体を検索し、引数の数を検査するために使うほか、
// - 構文解析の時点で、函数の有無や引数の数を検査するためにも使う。
//

const M: usize = usize::MAX;

const FUNC_WITH_ENV_TBL: [(&str,    // 函数名
                           fn(&Vec<&XSequence>, &EvalEnv) -> Result<XSequence, Box<Error>>,
                                    // 函数の実体
                           usize,   // 引数の数: min
                           usize);  // 引数の数: max
                           2] = [
// 16
    ( "position",  fn_position,  0, 0 ),
    ( "last",      fn_last,      0, 0 ),
    // funcname, func, min_args, max_args
];

const FUNC_TBL: [(&str,             // 函数名
                  fn(&Vec<&XSequence>) -> Result<XSequence, Box<Error>>,
                                    // 函数の実体
                  usize,            // 引数の数: min
                  usize,            // 引数の数: max
                  bool);            // 引数が不足しているとき文脈ノードを補う
                  25] = [
// 2
    ( "string",                 fn_string,                 1, 1, true ),
// 6.4
    ( "ceiling",                fn_ceiling,                1, 1, false ),
    ( "floor",                  fn_floor,                  1, 1, false ),
    ( "round",                  fn_round,                  1, 1, false ),
// 7.3
    ( "compare",                fn_compare,                2, 3, false ),
// 7.4
    ( "concat",                 fn_concat,                 2, M, false ),
    ( "substring",              fn_substring,              2, 3, false ),
    ( "string-length",          fn_string_length,          1, 1, true ),
    ( "normalize-space",        fn_normalize_space,        1, 1, true ),
    ( "translate",              fn_translate,              3, 3, false ),
// 7.5
    ( "contains",               fn_contains,               2, 3, false ),
    ( "starts-with",            fn_starts_with,            2, 3, false ),
    ( "substring-before",       fn_substring_before,       2, 3, false ),
    ( "substring-after",        fn_substring_after,        2, 3, false ),
// 9.1
    ( "true",                   fn_true,                   0, 0, false ),
    ( "false",                  fn_false,                  0, 0, false ),
// 9.3
    ( "not",                    fn_not,                    1, 1, false ),
// 14
    ( "name",                   fn_name,                   1, 1, true ),
    ( "local-name",             fn_local_name,             1, 1, true ),
    ( "namespace-uri",          fn_namespace_uri,          1, 1, true ),
    ( "number",                 fn_number,                 1, 1, true ),
    ( "lang",                   fn_lang,                   1, 2, true ),
// 15.1
    ( "boolean",                fn_boolean,                1, 1, false ),
// 15.3
// 15.4
    ( "count",                  fn_count,                  1, 1, false ),
    ( "sum",                    fn_sum,                    1, 2, false ),
    // funcname, func, min_args, max_args, default_is_context_node_set
    // ( "id",                    fn_id,                     1, 2, true ),
];

// ---------------------------------------------------------------------
//
pub fn check_function_spec(func_name: &str, num_args: usize) -> bool {
    let mut found = false;
    let mut fn_min_args = 0;
    let mut fn_max_args = 0;
    let mut fn_default_is_context_node_set = false;

    // 環境情報を参照する函数。
    for (name, _func, min_args, max_args) in FUNC_WITH_ENV_TBL.iter() {
        if &func_name == name {
            found = true;
            fn_min_args = *min_args;
            fn_max_args = *max_args;
        }
    }

    // 引数を取る函数。
    for (name, _func, min_args, max_args, default_is_context_node_set) in FUNC_TBL.iter() {
        if &func_name == name {
            found = true;
            fn_min_args = *min_args;
            fn_max_args = *max_args;
            fn_default_is_context_node_set = *default_is_context_node_set;
        }
    }

    if ! found {
        return false;
    }

    if fn_default_is_context_node_set {
        if num_args + 1 < fn_min_args {
            return false;
        }
    } else {
        if num_args < fn_min_args {
            return false;
        }
    }
    if fn_max_args < num_args {
        return false;
    }

    return true;
}

// ---------------------------------------------------------------------
// args: XNodeFunctionCallノードの右にたどった各XNodeArgumentTopノードの、
//       評価結果の配列
//
pub fn evaluate_function(func_name: &str, args: &Vec<XSequence>,
                xseq: &XSequence,
                eval_env: &EvalEnv) -> Result<XSequence, Box<Error>> {

    let mut found = false;
    let mut fn_env: Option<&fn(&Vec<&XSequence>, &EvalEnv) -> Result<XSequence, Box<Error>>> = None;
    let mut fn_arg: Option<&fn(&Vec<&XSequence>) -> Result<XSequence, Box<Error>>> = None;
    let mut fn_min_args = 0;
    let mut fn_max_args = 0;
    let mut fn_default_is_context_node_set = false;

    // 環境情報を参照する函数。
    for (name, func, min_args, max_args) in FUNC_WITH_ENV_TBL.iter() {
        if &func_name == name {
            found = true;
            fn_env = Some(func);
            fn_min_args = *min_args;
            fn_max_args = *max_args;
        }
    }

    // 引数を取る函数。
    for (name, func, min_args, max_args, default_is_context_node_set) in FUNC_TBL.iter() {
        if &func_name == name {
            found = true;
            fn_arg = Some(func);
            fn_min_args = *min_args;
            fn_max_args = *max_args;
            fn_default_is_context_node_set = *default_is_context_node_set;
        }
    }

    if ! found {
        return Err(cant_occur!("{}: この函数は未実装 (構文解析時の検査漏れ)。",
            func_name));
    }

    // 最後の引数が欠けていて、その既定値が文脈ノードである場合、これを補う。
    let arg_xseq = xseq.clone();
    let mut fn_args: Vec<&XSequence> = vec!{};
    for arg in args.iter() {
        fn_args.push(arg);
    }

    if fn_default_is_context_node_set && args.len() == fn_max_args - 1 {
        fn_args.push(&arg_xseq);
    }

    if fn_args.len() < fn_min_args {
        return Err(cant_occur!("{}: 引数が不足 (min: {}) (構文解析時の検査漏れ)。",
            func_name, fn_min_args));
    }
    if fn_max_args < fn_args.len() {
        return Err(cant_occur!("{}: 引数が過剰 (max: {}) (構文解析時の検査漏れ)。",
            func_name, fn_max_args));
    }

    // 実行する。
    if let Some(func) = fn_env {
        return func(&fn_args, eval_env);
    } else if let Some(func) = fn_arg {
        return func(&fn_args);
    } else {
        return Err(cant_occur!("{}: 該当する函数がない (構文解析時の検査漏れ)。",
            func_name));
    }
}

// ---------------------------------------------------------------------
// 2 Accessors
//      node-name
//      nilled
//      string
//      data
//      base-uri
//      document-uri
//
// ---------------------------------------------------------------------
// 2.3 fn:string
// fn:string() as xs:string
// fn:string($arg as item()?) as xs:string
//      空シーケンス => 空文字列
//      ノード => 文字列値
//      原子値 => $arg cast as xs:string
//
fn fn_string(args: &Vec<&XSequence>) -> Result<XSequence, Box<Error>> {
    if args[0].is_empty() {
        return Ok(new_singleton_string(&""));
    }

    let item = args[0].get_singleton_item()?;
    let result = item.cast_as_string();
    return Ok(new_singleton_string(&result));
}

// ---------------------------------------------------------------------
// 3 Error Function
//

// ---------------------------------------------------------------------
// 4 Trace Function
//

// ---------------------------------------------------------------------
// 5 Constructor Functions
//

// ---------------------------------------------------------------------
// 6 Functions and Operators on Numerics
//

// ---------------------------------------------------------------------
// 6.2 Operators on Numeric Values
//
// ---------------------------------------------------------------------
// 6.3 Comparison Operators on Numeric Values
//
// ---------------------------------------------------------------------
// 6.4 Functions on Numeric Values
//        abs
//        ceiling
//        floor
//        round
//        round_half_to_even
//
// ---------------------------------------------------------------------
// 6.4.2 fn:ceiling
// fn:ceiling($arg as numeric?) as numeric?
//      空シーケンス => 空シーケンス
//
fn fn_ceiling(args: &Vec<&XSequence>) -> Result<XSequence, Box<Error>> {
    return fn_numeric_unary(args,
                |a| { a },
                |a| { a.ceil() },
                |a| { ceil_x(a) });
}

// ---------------------------------------------------------------------
// 6.4.3 fn:floor
// fn:floor($arg as numeric?) as numeric?
//      空シーケンス => 空シーケンス
//
fn fn_floor(args: &Vec<&XSequence>) -> Result<XSequence, Box<Error>> {
    return fn_numeric_unary(args,
                |a| { a },
                |a| { a.floor() },
                |a| { floor_x(a) });
}

// ---------------------------------------------------------------------
// 6.4.4 fn:round
// fn:round($arg as numeric?) as numeric?
//      空シーケンス => 空シーケンス
//
fn fn_round(args: &Vec<&XSequence>) -> Result<XSequence, Box<Error>> {
    return fn_numeric_unary(args,
                |a| { a },
                |a| { (a + 0.5).floor() },
                        // a.round() ではない。
                        // round(-2.5) => -2 となるようにするため。
                |a| { round_x(a) });
}

// ---------------------------------------------------------------------
// ceil_x/floor_x/round_x: 天井/床/四捨五入だが、負のゼロの扱いが特殊。
//
fn ceil_x(num: f64) -> f64 {
    if (num == 0.0 && num.signum() == -1.0) ||          // 負のゼロ
       (-1.0 < num && num < 0.0) {
        return 1.0 / f64::NEG_INFINITY;                 // 負のゼロ
    } else {
        return num.ceil();
    }
}

fn floor_x(num: f64) -> f64 {
    if num == 0.0 && num.signum() == -1.0 {             // 負のゼロ
        return 1.0 / f64::NEG_INFINITY;                 // 負のゼロ
    } else {
        return num.floor();
    }
}

fn round_x(num: f64) -> f64 {
    if (num == 0.0 && num.signum() == -1.0) ||          // 負のゼロ
       (-0.5 <= num && num < 0.0) {
        return 1.0 / f64::NEG_INFINITY;                 // 負のゼロ
    } else {
        return (num + 0.5).floor();
    }
}

// ---------------------------------------------------------------------
//
fn fn_numeric_unary<FINT, FDEC, FDBL>(args: &Vec<&XSequence>,
        mut int_op: FINT, mut dec_op: FDEC, mut dbl_op: FDBL) -> Result<XSequence, Box<Error>>
        where FINT: FnMut(i64) -> i64,
              FDEC: FnMut(f64) -> f64,
              FDBL: FnMut(f64) -> f64 {
    if let Ok(arg) = args[0].get_singleton_item() {
        match arg {
            XItem::XIInteger{value: arg} => {
                return Ok(new_singleton_integer(int_op(arg)));
            },
            XItem::XIDecimal{value: arg} => {
                return Ok(new_singleton_decimal(dec_op(arg)));
            },
            XItem::XIDouble{value: arg} => {
                return Ok(new_singleton_double(dbl_op(arg)));
            },
            _ => {},
        }
    }
    return Ok(new_xsequence());
}

// ---------------------------------------------------------------------
// 7 Functions on Strings
//

// ---------------------------------------------------------------------
// 7.2 Functions to Assemble and Disassemble Strings
//

// ---------------------------------------------------------------------
// 7.3 Equality and Comparison of Strings
//          compare
//          codepoint_equal
//
// ---------------------------------------------------------------------
// 7.3.2 fn:compare
// fn:compare($comparand1 as xs:string?,
//            $comparand2 as xs:string?) as xs:integer?
// fn:compare($comparand1 as xs:string?,
//            $comparand2 as xs:string?,
//            $collation as xs:string) as xs:integer?
// いずれかの引数が空シーケンスの場合、空シーケンスを返す。
// 第3引数 $collation がある場合の比較は未実装。
//
pub fn fn_compare(args: &Vec<&XSequence>) -> Result<XSequence, Box<Error>> {

    if args.len() != 2 {
        panic!("fn_compare: args.len() != 2.");
            // 実際には第3引数 collation も指定できる。
    }
    if args[0].is_empty() || args[1].is_empty() {
        return Ok(new_xsequence());
    }

    let lhs = args[0].get_singleton_string()?;
    let rhs = args[1].get_singleton_string()?;
    if lhs < rhs {
        return Ok(new_singleton_integer(-1));
    } else if lhs == rhs {
        return Ok(new_singleton_integer(0));
    } else {
        return Ok(new_singleton_integer(1));
    }
}

// ---------------------------------------------------------------------
// 7.4 Functions on String Values
//      concat
//      string_join
//      substring
//      string_length
//      normalize_space
//      normalize_unicode
//      upper_case
//      lower_case
//      translate
//      encode_for_uri
//      iri_to_uri
//      escape_html_uri
//
// ---------------------------------------------------------------------
// 7.4.1 fn:concat
// fn:concat($arg1 as xs:anyAtomicType?,
//           $arg2 as xs:anyAtomicType?,
//           ... ) as xs:string
//
//                  引数がすべて空シーケンスの場合、空文字列を返す。
//
fn fn_concat(args: &Vec<&XSequence>) -> Result<XSequence, Box<Error>> {
    let mut val = String::new();
    for arg in args.iter() {
        if ! arg.is_empty() {
            val += &arg.get_singleton_item()?.cast_as_string();
        }
    }
    return Ok(new_singleton_string(&val));
}

// ---------------------------------------------------------------------
// 7.4.3 fn:substring
// fn:substring($sourceString as xs:string?,
//              $startingLoc as xs:double) as xs:string
// fn:substring($sourceString as xs:string?,
//              $startingLoc as xs:double,
//              $length as xs:double) as xs:string
//
//  - source_stringが空シーケンスであれば空文字列を返す。
//  - starting_loc や length がNaNならば空文字列。
//  - starting_loc = -∞ のとき: lengthが有限ならばe = -∞なので空文字列、
//              length = ∞ならば - ∞ + ∞ = NaNなのでやはり空文字列。
//  - starting_loc = ∞ のとき: lengthにかかわらずe = ∞なので空文字列、
//
fn fn_substring(args: &Vec<&XSequence>) -> Result<XSequence, Box<Error>> {

    if args[0].len() == 0 {
        return Ok(new_singleton_string(&""));
    }

    let source_string = args[0].get_singleton_string()?;
    let sv: Vec<char> = source_string.chars().collect();
    let sv_len = usize_to_i64(sv.len());

    let starting_loc = args[1].get_singleton_item()?.cast_as_double()?;
    if starting_loc.is_nan() || starting_loc.is_infinite() {
        return Ok(new_singleton_string(&""));
    }
    let beg_pos = round_x(starting_loc) as i64;     // 有限値
    let mut b = beg_pos - 1;
    if b < 0 {
        b = 0;
    }
    if sv.len() <= b as usize {
        b = sv_len;
    }
    let mut e = i64::MAX;
    if args.len() == 2 {
        e = sv_len;
    } else {
        let length = args[2].get_singleton_item()?.cast_as_double()?;
        if length.is_nan() || length.is_sign_negative() {
            return Ok(new_singleton_string(&""));
        }
        let len_str = f64_to_i64(round_x(length));      // 非負値 (+∞を含む)
        if len_str != i64::MAX {
            e = beg_pos + len_str - 1;
        }
        if e < b {
            e = b;
        }
        if sv.len() as i64 <= e {
            e = sv_len;
        }
    }
    let mut result = String::new();
    for i in b..e {
        result.push(sv[i as usize]);
    }
    return Ok(new_singleton_string(&result));
}

// ---------------------------------------------------------------------
// 7.4.4 fn:string-length
// fn:string-length() as xs:integer
// fn:string-length($arg as xs:string?) as xs:integer
//
fn fn_string_length(args: &Vec<&XSequence>) -> Result<XSequence, Box<Error>> {
    if args[0].is_empty() {
        return Ok(new_singleton_integer(0));
    }

    let arg = args[0].get_singleton_string()?;
    let v: Vec<char> = arg.chars().collect();
    let length = v.len();           // バイト長でなく文字数。
    return Ok(new_singleton_integer(usize_to_i64(length)));
}

// ---------------------------------------------------------------------
// 7.4.5 fn:normalize-space
// fn:normalize-space() as xs:integer
// fn:normalize-space($arg as xs:string?) as xs:integer
//
fn fn_normalize_space(args: &Vec<&XSequence>) -> Result<XSequence, Box<Error>> {
    if args[0].is_empty() {
        return Ok(new_singleton_string(&""));
    }

    let arg = args[0].get_singleton_string()?;
    let v: Vec<&str> = arg.split_whitespace().collect();
    let mut result = String::new();
    for t in v.iter() {
        result += t;
    }
    return Ok(new_singleton_string(&result));
}

// ---------------------------------------------------------------------
// 7.4.9 fn:translate
// fn:translate($arg as xs:string?,
//              $mapString as xs:string,
//              $transString as xs:string) as xs:string
//
fn fn_translate(args: &Vec<&XSequence>) -> Result<XSequence, Box<Error>> {

    if args[0].is_empty() {
        return Ok(new_singleton_string(&""));
    }

    let str = args[0].get_singleton_string()?;
    let from = args[1].get_singleton_string()?;
    let to = args[2].get_singleton_string()?;

    let str_runes: Vec<char> = str.chars().collect();
    let from_runes: Vec<char> = from.chars().collect();
    let to_runes: Vec<char> = to.chars().collect();
    let mut result = String::new();
    for ch in str_runes.iter() {
        let mut index:usize = usize::MAX;
        for (i, c) in from_runes.iter().enumerate() {
            if ch == c {
                index = i;
            }
        }
        if index != usize::MAX {
            if index < to_runes.len() {
                result.push(to_runes[index]);
            }
        } else {
            result.push(*ch);
        }
    }

    return Ok(new_singleton_string(&result));
}


// ---------------------------------------------------------------------
// 7.5 Functions Based on Substring Matching
//      contains
//      starts_with
//      ends_with
//      substring_before
//      substring_after
//
// ---------------------------------------------------------------------
// 7.5.1 fn:contains
// fn:contains($arg1 as xs:string?, $arg2 as xs:string?) as xs:boolean
// fn:contains($arg1 as xs:string?,
//             $arg2 as xs:string?,
//             $collation as xs:string) as xs:boolean
//
fn fn_contains(args: &Vec<&XSequence>) -> Result<XSequence, Box<Error>> {
    let mut arg1 = String::new();
    if ! args[0].is_empty() {
        arg1 = args[0].get_singleton_string()?;
    }

    let mut arg2 = String::new();
    if ! args[1].is_empty() {
        arg2 = args[1].get_singleton_string()?;
    }

    let b = (&arg1).contains(&arg2);
    return Ok(new_singleton_boolean(b));
}

// ---------------------------------------------------------------------
// 7.5.2 fn:start-with
// fn:start-with($arg1 as xs:string?, $arg2 as xs:string?) as xs:boolean
// fn:start-with($arg1 as xs:string?,
//               $arg2 as xs:string?,
//               $collation as xs:string) as xs:boolean
//
fn fn_starts_with(args: &Vec<&XSequence>) -> Result<XSequence, Box<Error>> {
    let mut arg1 = String::new();
    if ! args[0].is_empty() {
        arg1 = args[0].get_singleton_string()?;
    }

    let mut arg2 = String::new();
    if ! args[1].is_empty() {
        arg2 = args[1].get_singleton_string()?;
    }

    let b = (&arg1).starts_with(&arg2);
    return Ok(new_singleton_boolean(b));
}

// ---------------------------------------------------------------------
// 7.5.4 fn:substring-before
// fn:substring-before($arg1 as xs:string?, $arg2 as xs:string?) as xs:string
// fn:substring-before($arg1 as xs:string?,
//                     $arg2 as xs:string?,
//                     $collation as xs:string) as xs:string
//
fn fn_substring_before(args: &Vec<&XSequence>) -> Result<XSequence, Box<Error>> {
    let mut arg1 = String::new();
    if ! args[0].is_empty() {
        arg1 = args[0].get_singleton_string()?;
    }

    let mut arg2 = String::new();
    if ! args[1].is_empty() {
        arg2 = args[1].get_singleton_string()?;
    }

    let v: Vec<&str> = (&arg1).splitn(2, &arg2).collect();
    let mut result = String::new();
    if 2 <= v.len() {
        result = v[0].to_string();
    }
    return Ok(new_singleton_string(&result));
}

// ---------------------------------------------------------------------
// 7.5.5 fn:substring-after
// fn:substring-after($arg1 as xs:string?, $arg2 as xs:string?) as xs:string
// fn:substring-after($arg1 as xs:string?,
//                    $arg2 as xs:string?,
//                    $collation as xs:string) as xs:string
//
fn fn_substring_after(args: &Vec<&XSequence>) -> Result<XSequence, Box<Error>> {
    let mut arg1 = String::new();
    if ! args[0].is_empty() {
        arg1 = args[0].get_singleton_string()?;
    }

    let mut arg2 = String::new();
    if ! args[1].is_empty() {
        arg2 = args[1].get_singleton_string()?;
    }

    let v: Vec<&str> = (&arg1).splitn(2, &arg2).collect();
    let mut result = String::new();
    if 2 <= v.len() {
        result = v[1].to_string();
    }
    return Ok(new_singleton_string(&result));
}

// ---------------------------------------------------------------------
// 7.6 String Functions that Use Pattern Matching
//

// ---------------------------------------------------------------------
// 8 Functions on anyURI
//

// ---------------------------------------------------------------------
// 9 Functions and Operators on Boolean Values
//

// ---------------------------------------------------------------------
// 9.1 Additional Boolean Constructor Functions
//
// ---------------------------------------------------------------------
// 9.1 fn:true
// fn:true() as xs:boolean
//
fn fn_true(_args: &Vec<&XSequence>) -> Result<XSequence, Box<Error>> {
    return Ok(new_singleton_boolean(true));
}

// ---------------------------------------------------------------------
// 9.2 fn:false
// fn:false() as xs:boolean
//
fn fn_false(_args: &Vec<&XSequence>) -> Result<XSequence, Box<Error>> {
    return Ok(new_singleton_boolean(false));
}

// ---------------------------------------------------------------------
// 9.2 Operators on Boolean Values
//
// ---------------------------------------------------------------------
// 9.3 Functions on Boolean Values
//
// ---------------------------------------------------------------------
// 9.3.1 fn:not
// fn:not($arg as item()*) as xs:boolean
//      実効ブール値の否定を返す。
//
pub fn fn_not(args: &Vec<&XSequence>) -> Result<XSequence, Box<Error>> {
    let b = args[0].effective_boolean_value()?;
    return Ok(new_singleton_boolean(! b));
}

// ---------------------------------------------------------------------
// 10 Functions and Operators on Durations, Dates and Times
// ---------------------------------------------------------------------
// 11 Functions Related to QNames
// ---------------------------------------------------------------------
// 12 Operators on base64Binary and hexBinary
// ---------------------------------------------------------------------
// 13 Operators on NOTATION

// ---------------------------------------------------------------------
// 14 Functions and Operators on Nodes
//          name
//          local_name
//          namespace_uri
//          number
//          lang
//          root
//
// ---------------------------------------------------------------------
// 14.1 fn:name
// fn:name() as xs:string
// fn:name($arg as node()?) as xs:string
//
fn fn_name(args: &Vec<&XSequence>) -> Result<XSequence, Box<Error>> {
    if args[0].is_empty() {
        return Ok(new_singleton_string(&""));
    }
    if let Ok(node) = args[0].get_singleton_node() {
        let name = node.name();
        return Ok(new_singleton_string(&name));
    }
    return Err(dynamic_error!("name(): Item is not a node"));
}

// ---------------------------------------------------------------------
// 14.2 fn:local-name
// fn:local-name() as xs:string
// fn:local-name($arg as node()?) as xs:string
//
fn fn_local_name(args: &Vec<&XSequence>) -> Result<XSequence, Box<Error>> {
    if args[0].is_empty() {
        return Ok(new_singleton_string(&""));
    }
    if let Ok(node) = args[0].get_singleton_node() {
        let name = node.local_name();
        return Ok(new_singleton_string(&name));
    }
    return Err(dynamic_error!("local-name(): Item is not a node"));
}

// ---------------------------------------------------------------------
// 14.3 fn:namespace-uri
// fn:namespace-uri() as xs:anyURI
// fn:namespace-uri($arg as node()?) as xs:anyURI
//
fn fn_namespace_uri(args: &Vec<&XSequence>) -> Result<XSequence, Box<Error>> {
    if args[0].is_empty() {
        return Ok(new_singleton_string(&""));
    }
    if let Ok(node) = args[0].get_singleton_node() {
        let name = node.namespace_uri();
        return Ok(new_singleton_string(&name));
    }
    return Err(dynamic_error!("namespace-uri(): Item is not a node"));
}

// ---------------------------------------------------------------------
// 14.4 fn:number
// fn:number() as xs:double
// fn:number($arg as xs:anyAtomicType?) as xs:double
//
fn fn_number(args: &Vec<&XSequence>) -> Result<XSequence, Box<Error>> {
    if args[0].is_empty() {
        return Ok(new_singleton_double(f64::NAN));
    }
    let mut result = 0.0;
    if let Ok(arg) = args[0].get_singleton_item() {
        result = arg.cast_as_double()?;
    }
    return Ok(new_singleton_double(result));
}

// ---------------------------------------------------------------------
// 14.5 fn:lang
// fn:lang($testlang as xs:string?) as xs:boolean
// fn:lang($testlang as xs:string?, $node as node()) as xs:boolean
//
fn fn_lang(args: &Vec<&XSequence>) -> Result<XSequence, Box<Error>> {
    if args[0].is_empty() {
        return Ok(new_singleton_string(&""));
    }
    let testlang = args[0].get_singleton_string()?;
    let node = args[1].get_singleton_node()?;

    let mut xmllang = String::new();
    for n in array_ancestor_or_self(&node).iter() {
                                // array_ancestor_or_self() !!!!!!!!!!!!
        if let Some(val) = n.attribute_value("xml:lang") {
            xmllang = val.to_lowercase();
            break;
        }
    }
    let v: Vec<&str> = xmllang.splitn(2, "-").collect();
    let result = v[0] == testlang;

    return Ok(new_singleton_boolean(result));
}

// ---------------------------------------------------------------------
// 15 Functions and Operators on Sequences
//

// ---------------------------------------------------------------------
// 15.1 General Functions and Operators on Sequences
//
// ---------------------------------------------------------------------
// 15.1.1 fn:boolean
// fn:boolean($arg as item()*) as xs:boolean
//      実効ブール値を返す。
//
fn fn_boolean(args: &Vec<&XSequence>) -> Result<XSequence, Box<Error>> {

    let b = args[0].effective_boolean_value()?;
    return Ok(new_singleton_boolean(b));
}

// ---------------------------------------------------------------------
// 15.2 Functions That Test the Cardinality of Sequences
//
// ---------------------------------------------------------------------
// 15.4 Aggregate Functions
//          count
//          avg
//          max
//          min
//          sum
//
// ---------------------------------------------------------------------
// 15.4.1 fn:count
// fn:count($arg as item()*) as xs:integer
//
fn fn_count(args: &Vec<&XSequence>) -> Result<XSequence, Box<Error>> {
    return Ok(new_singleton_integer(usize_to_i64(args[0].len())));
}

// ---------------------------------------------------------------------
// 15.4.5 fn:sum
// fn:sum($arg as xs:anyAtomicType*) as xs:anyAtomicType
// fn:sum($arg as xs:anyAtomicType*,
//        $zero as xs:anyAtomicType?) as xs:anyAtomicType?
//
fn fn_sum(args: &Vec<&XSequence>) -> Result<XSequence, Box<Error>> {
    if args[0].len() == 0 {
        if args.len() <= 1 {
            return Ok(new_singleton_integer(0));
        } else {
            return Ok(args[1].clone());
        }
    }

    let mut val = new_xitem_integer(0);
    for i in 0 .. args[0].len() {
        val = xitem_numeric_add(&val, args[0].get_item(i))?;
                        // 必要に応じて型の昇格をしながら加算していく。
    }
    return Ok(new_singleton(&val));
}

// ---------------------------------------------------------------------
// 15.5 Functions and Operators that Generate Sequences
//          id
//          idref
//          doc
//          doc_available
//          collection
//

// ---------------------------------------------------------------------
// 16 Context Functions
//
fn fn_position(_args: &Vec<&XSequence>, eval_env: &EvalEnv) -> Result<XSequence, Box<Error>> {
    return Ok(new_singleton_integer(usize_to_i64(eval_env.get_position())));
}

fn fn_last(_args: &Vec<&XSequence>, eval_env: &EvalEnv) -> Result<XSequence, Box<Error>> {
    return Ok(new_singleton_integer(usize_to_i64(eval_env.get_last())));
}

// ---------------------------------------------------------------------
// 17 Casting
// ---------------------------------------------------------------------
//

// =====================================================================
//
#[cfg(test)]
mod test {
//    use super::*;

    use xpath2::helpers::compress_spaces;
    use xpath2::helpers::subtest_xpath;
    use xpath2::helpers::subtest_eval_xpath;

    // -----------------------------------------------------------------
    // 2.3 fn:string
    //
    #[test]
    fn test_fn_string() {
        let xml = compress_spaces(r#"
<a base="base">
    string value
</a>
        "#);
        subtest_eval_xpath("fn_string", &xml, &[
            ( r#"string(37)"#, r#"("37")"# ),
            ( r#"string(37.3)"#, r#"("37.3")"# ),
            ( r#"string(true())"#, r#"("true")"# ),
            ( r#"string()"#, r#"("string value")"# ),   // 文脈ノードの文字列値
            ( r#"string(.)"#, r#"("string value")"# ),
            ( r#"string(/a)"#, r#"("string value")"# ),
            ( r#"string(/a/empty)"#, r#"("")"# ),
        ]);
    }

    // -----------------------------------------------------------------
    // 6.4.2 fn:ceiling
    //
    #[test]
    fn test_fn_ceiling() {
        let xml = compress_spaces(r#"
<a base="base">
</a>
        "#);
        subtest_eval_xpath("fn_ceiling", &xml, &[
            ( "ceiling(37)", "(37)" ),
            ( "ceiling(10.5)", "(11.0)" ),
            ( "ceiling(-10.5)", "(-10.0)" ),
            ( "ceiling(-0e0)", "(-0e0)" ),          // 負のゼロ -> 負のゼロ
            ( "ceiling(-0.2e0)", "(-0e0)" ),        // (-1, 0) -> 負のゼロ
        ]);
    }

    // -----------------------------------------------------------------
    // 6.4.3 fn:floor
    //
    #[test]
    fn test_fn_floor() {
        let xml = compress_spaces(r#"
<a base="base">
</a>
        "#);
        subtest_eval_xpath("fn_floor", &xml, &[
            ( "floor(37)", "(37)" ),
            ( "floor(10.5)", "(10.0)" ),
            ( "floor(-10.5)", "(-11.0)" ),
            ( "floor(0e0)", "(0e0)" ),            // 正のゼロ -> 正のゼロ
            ( "floor(-0e0)", "(-0e0)" ),          // 負のゼロ -> 負のゼロ
        ]);
    }

    // -----------------------------------------------------------------
    // 6.4.4 fn:round
    //
    #[test]
    fn test_fn_round() {
        let xml = compress_spaces(r#"
<a base="base">
</a>
        "#);
        subtest_eval_xpath("fn_round", &xml, &[
            ( "round(37)", "(37)" ),
            ( "round(2.5)", "(3.0)" ),
            ( "round(2.4999)", "(2.0)" ),
            ( "round(-2.5)", "(-2.0)" ),
                            // !! not the possible alternative, -3.0
            ( "round(-0e0)", "(-0e0)" ),            // 負のゼロ -> 負のゼロ
            ( "round(-0.3e0)", "(-0e0)" ),          // (-0.5, -0) -> 負のゼロ
        ]);
    }

    // -----------------------------------------------------------------
    // 7.3.2 fn:compare
    //
    #[test]
    fn test_fn_compare() {
        let xml = compress_spaces(r#"
<root base="base">
</root>
        "#);
        subtest_eval_xpath("fn_compare", &xml, &[
            ( r#"compare('abc', 'abc')"#, "(0)" ),
            ( r#"compare('abc', 'abx')"#, "(-1)" ),
        ]);
    }

    // -----------------------------------------------------------------
    // 7.4.1 fn:concat
    //
    #[test]
    fn test_fn_concat() {
        let xml = compress_spaces(r#"
<root base="base">
</root>
        "#);
        subtest_eval_xpath("fn_concat", &xml, &[
            ( r#"concat("あい")"#, "Syntax Error in XPath" ),   // 引数不足
            ( r#"concat("あい", "うえ")"#, r#"("あいうえ")"# ),
            ( r#"concat(123, 456, 789)"#, r#"("123456789")"# ),
            ( r#"concat((), "A", ())"#, r#"("A")"# ),
            ( r#"concat((), (), ())"#, r#"("")"# ),
        ]);
    }

    // -----------------------------------------------------------------
    // 7.4.3 fn:substring
    //
    #[test]
    fn test_fn_substring() {
        let xml = compress_spaces(r#"
<a base="base">
</a>
        "#);
        subtest_eval_xpath("fn_substring", &xml, &[
            ( r#"substring("ABCDE", 2, 3)"#, r#"("BCD")"# ),
            ( r#"substring("ABCDE", 2)"#, r#"("BCDE")"# ),
            ( r#"substring("ABCDE", 1.5, 2.6)"#, r#"("BCD")"# ),
            ( r#"substring("ABCDE", 0, 3)"#, r#"("AB")"# ),
            ( r#"substring("ABCDE", 5, -3)"#, r#"("")"# ),
            ( r#"substring("ABCDE", -3, 5)"#, r#"("A")"# ),
            ( r#"substring("ABCDE", 0 div 0e0, 3)"#, r#"("")"# ),
            ( r#"substring("ABCDE", 1, 0 div 0e0)"#, r#"("")"# ),

            ( r#"substring("ABCDE", -42, 1 div 0e0)"#, r#"("ABCDE")"# ),
            ( r#"substring("ABCDE", -1 div 0e0, 1 div 0e0)"#, r#"("")"# ),

            ( r#"substring("あいうえお", 2, 3)"#, r#"("いうえ")"# ),
            ( r#"substring("あいうえお", 2)"#, r#"("いうえお")"# ),
            ( r#"substring("あいうえお", 1.5, 2.6)"#, r#"("いうえ")"# ),
            ( r#"substring("あいうえお", 0, 3)"#, r#"("あい")"# ),
        ]);
    }

    // -----------------------------------------------------------------
    // 7.4.4 fn:string-length
    //
    #[test]
    fn test_fn_string_length() {
        let xml = compress_spaces(r#"
<a base="base">
</a>
        "#);
        subtest_eval_xpath("fn_string_length", &xml, &[
            ( r#"string-length('')"#, "(0)" ),
            ( r#"string-length('かきくけこ')"#, "(5)" ),
        ]);
    }

    // -----------------------------------------------------------------
    // 7.4.5 fn:normalize-space
    //
    #[test]
    fn test_fn_normalize_space() {
        let xml = compress_spaces(r#"
<a base="base">
</a>
        "#);
        subtest_eval_xpath("fn_normalize_space", &xml, &[
            ( r#"normalize-space('')"#, r#"("")"# ),
            ( r#"normalize-space(' abc  def ')"#, r#"("abcdef")"# ),
        ]);
    }

    // -----------------------------------------------------------------
    // 7.4.9 fn:translate
    //
    #[test]
    fn test_fn_translate() {
        let xml = compress_spaces(r#"
<a base="base">
</a>
        "#);
        subtest_eval_xpath("fn_translate", &xml, &[
            ( r#"translate("bar", "abc", "ABC")"#, r#"("BAr")"# ),
            ( r#"translate("---aaa---", "abc-", "ABC")"#, r#"("AAA")"# ),
        ]);
    }

    // -----------------------------------------------------------------
    // 7.5.1 fn:contains
    //
    #[test]
    fn test_fn_contains() {
        let xml = compress_spaces(r#"
<a base="base">
</a>
        "#);
        subtest_eval_xpath("fn_contains", &xml, &[
            ( r#"contains("かきくけこ", "きく")"#, "(true)" ),
            ( r#"contains("かきくけこ", "たち")"#, "(false)" ),
            ( r#"contains("", "たち")"#, "(false)" ),
            ( r#"contains("かきくけこ", "")"#, "(true)" ),
            ( r#"contains("", "")"#, "(true)" ),
        ]);
    }

    // -----------------------------------------------------------------
    // 7.5.2 fn:starts-with
    //
    #[test]
    fn test_fn_starts_with() {
        let xml = compress_spaces(r#"
<a base="base">
</a>
        "#);
        subtest_eval_xpath("fn_starts_with", &xml, &[
            ( r#"starts-with("かきくけこ", "かき")"#, "(true)" ),
            ( r#"starts-with("かきくけこ", "たち")"#, "(false)" ),
            ( r#"starts-with("", "たち")"#, "(false)" ),
            ( r#"starts-with("かきくけこ", "")"#, "(true)" ),
            ( r#"starts-with("", "")"#, "(true)" ),
        ]);
    }

    // -----------------------------------------------------------------
    // 7.5.4 fn:substring-before
    //
    #[test]
    fn test_fn_substring_before() {
        let xml = compress_spaces(r#"
<a base="base">
</a>
        "#);
        subtest_eval_xpath("fn_substring_before", &xml, &[
            ( r#"substring-before("1999/04/01", "/")"#, r#"("1999")"# ),
            ( r#"substring-before("1999/04/01", "X")"#, r#"("")"# ),
        ]);
    }

    // -----------------------------------------------------------------
    // 7.5.5 fn:substring-after
    //
    #[test]
    fn test_fn_substring_after() {
        let xml = compress_spaces(r#"
<a base="base">
</a>
        "#);
        subtest_eval_xpath("fn_substring_after", &xml, &[
            ( r#"substring-after("1999/04/01", "/")"#, r#"("04/01")"# ),
            ( r#"substring-after("1999/04/01", "X")"#, r#"("")"# ),
        ]);
    }

    // -----------------------------------------------------------------
    // 14.3 fn:namespace-uri
    //
    #[test]
    fn test_namespace_uri() {
        let xml = compress_spaces(r#"
<?xml version='1.0' encoding='UTF-8'?>
<xroot xmlns:amr='http://amr.jp/amr' xmlns='http://amr.jp/default'>
    <amr:case1 />
    <case2 />
    <file xmlns='http://amr.jp/subdefault'>
        <amr:case3 />
        <xxx:case4 />
        <case5 />
    </file>
    <a base="base">
        <sel img="A" ans="http://amr.jp/amr" />
        <sel img="B" ans="http://amr.jp/default" />
        <sel img="C" ans="http://amr.jp/subdefault" />
        <sel img="D" ans="" />
    </a>
</xroot>
        "#);

        subtest_xpath("namespace_uri", &xml, false, &[
            ( "//a/sel[@ans = namespace-uri(/xroot//amr:case1)]", "A" ),
            ( "//a/sel[@ans = namespace-uri(/xroot//case2)]", "B" ),
            ( "//a/sel[@ans = namespace-uri(/xroot//amr:case3)]", "A" ),
            ( "//a/sel[@ans = namespace-uri(/xroot//xxx:case4)]", "D" ),
            ( "//a/sel[@ans = namespace-uri(/xroot//case5)]", "C" ),
        ]);
    }

    // -----------------------------------------------------------------
    // 14.1 fn:name
    //
    #[test]
    fn test_fn_name() {
        let xml = compress_spaces(r#"
<root base="base">
    <para id="A"/>
</root>
        "#);
        subtest_eval_xpath("fn_name", &xml, &[
            ( "name()", r#"("root")"# ),
            ( "name(/root/*[1])", r#"("para")"# ),
            ( "name(123)", "Dynamic Error" ),
        ]);
    }

    // -----------------------------------------------------------------
    // 14.5 fn:lang
    //
    #[test]
    fn test_fn_lang() {
        let xml = compress_spaces(r#"
<?xml version='1.0' encoding='UTF-8'?>
<xroot xmlns='http://amr.jp/default'>
    <para id="A" xml:lang="en"/>
    <div id="B" xml:lang="en">
        <para id="C"/>
    </div>
    <para id="D" xml:lang="EN"/>
    <para id="E" xml:lang="en-us"/>
    <para id="F" />
    <a base="base">
        <sel img="z0" ans="0" />
        <sel img="z1" ans="1" />
    </a>
</xroot>
        "#);
        subtest_eval_xpath("fn_lang", &xml, &[
            ( "//para[@id='A'][lang('en')]", r#"(<para id="A" xml:lang="en">)"# ),
            ( "//para[@id='A'][lang('ja')]", r#"()"# ),

            ( "count(//para[@id='A'][lang('en')])", "(1)" ),
            ( "count(//div[@id='B'][lang('en')])", "(1)" ),
            ( "count(//para[@id='C'][lang('en')])", "(1)" ),
            ( "count(//para[@id='D'][lang('en')])", "(1)" ),
            ( "count(//para[@id='E'][lang('en')])", "(1)" ),
            ( "count(//para[@id='F'][lang('en')])", "(0)" ),
            ( "count(//para[@id='A'][lang('ja')])", "(0)" ),
        ]);
    }

    // -----------------------------------------------------------------
    // 15.4.5 fn:sum
    //
    #[test]
    fn test_fn_sum() {
        let xml = compress_spaces(r#"
<root base="base">
</root>
        "#);
        subtest_eval_xpath("fn_sum", &xml, &[
            ( "sum((1, 2, 3))", "(6)" ),
            ( "sum((1.5, 2.5, 3))", "(7.0)" ),
            ( "sum((1, 2, 3), (99))", "(6)" ),
            ( "sum(())", "(0)" ),
        ]);
    }

    // -----------------------------------------------------------------
    // 16.1 fn:position
    //
    #[test]
    fn test_fn_position() {
        let xml = compress_spaces(r#"
<root>
    <a id="1"/>
    <a id="2">
        <b id="1">
            <c id="1"/>
            <c id="2"/>
            <c id="3"/>
        </b>
        <b id="2"/>
            <a id="x1"/>
            <a id="x2"/>
            <a id="x3"/>
        <b id="3" base="base"/>
    </a>
    <a id="3" />
</root>
        "#);
        subtest_eval_xpath("fn_position", &xml, &[
            ( "position()", "(0)" ),
            ( "/root/a[position() = 2]", r#"(<a id="2">)"# ),
            ( "/root/a[not(position() = 2)]", r#"(<a id="1">, <a id="3">)"# ),
            ( "/root/a[position()=3 or position()=2]", r#"(<a id="2">, <a id="3">)"# ),
            ( "/root/a[position()=2]/b[position()=1]", r#"(<b id="1">)"# ),
            ( "/root/a[position()=2]/b[position()=1]/c[position()=3]", r#"(<c id="3">)"# ),
            ( "/root/a[position()=2], position()", r#"(<a id="2">, 0)"# ),
            ( "//a[position()=2]", r#"(<a id="2">, <a id="x2">)"# ),
            ( ".[position()=1]", r#"(<b id="3" base="base">)"# ),
            ( ".[position()=3]", r#"()"# ),
        ]);
    }

    // -----------------------------------------------------------------
    // 16.2 fn:last
    //
    #[test]
    fn test_fn_last() {
        let xml = compress_spaces(r#"
<root img="basic" base="base">
    <a id="1" />
    <a id="2">
        <b id="1">
            <c id="1"/>
            <c id="2"/>
            <c id="3"/>
        </b>
        <b id="2"/>
        <b id="3"/>
    </a>
    <a id="3" />
</root>
        "#);
        subtest_eval_xpath("fn_last", &xml, &[
            ( "/root/a[last()]", r#"(<a id="3">)"# ),
            ( "/root/a[position()=last()-1]", r#"(<a id="2">)"# ),
            ( "/root/a[position()=last()-1]/b[position()=last()-2]", r#"(<b id="1">)"# ),
            ( "/root/a[position()=last()-1]/b[position()=last()-2]/c[position()=last()]", r#"(<c id="3">)"# ),
        ]);
    }
}
