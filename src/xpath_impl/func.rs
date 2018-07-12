//
// xpath_impl/func.rs
//
// amxml: XML processor with XPath.
// Copyright (C) 2018 KOYAMA Hiro <tac@amris.co.jp>
//

use std::error::Error;
use std::f64;
use std::i64;
use std::usize;

use xmlerror::*;
use xpath_impl::eval::*;
use xpath_impl::xitem::*;
use xpath_impl::xsequence::*;

// ---------------------------------------------------------------------
//
fn usize_to_i64(n: usize) -> i64 {
    return n as i64;
}

// ---------------------------------------------------------------------
// 函数表。
// - 実行時に、函数の実体を検索し、引数の数を検査するために使うほか、
// - 構文解析の時点で、函数の有無や引数の数を検査するためにも使う。
//

const M: usize = usize::MAX;

const FUNC_WITH_ENV_TBL: [(
        &str,                                       // 函数名
        fn(&Vec<&XSequence>, &mut EvalEnv) -> Result<XSequence, Box<Error>>,
                                                    // 函数の実体
        usize,                                      // 引数の数: min
        usize);                                     // 引数の数: max
        4] = [
// 15
    ( "fn:position",  fn_position,  0, 0 ),
    ( "fn:last",      fn_last,      0, 0 ),
// 16.2
    ( "fn:for-each",  fn_for_each,  2, 2 ),
    ( "fn:filter",    fn_filter,    2, 2 ),

];

const FUNC_TBL: [(
        &str,                       // 函数名
        fn(&Vec<&XSequence>) -> Result<XSequence, Box<Error>>,
                                    // 函数の実体
        usize,                      // 引数の数: min
        usize,                      // 引数の数: max
        bool);                      // 引数が不足しているとき文脈ノードを補う
        47] = [
// 2
    ( "fn:string",                 fn_string,                 1, 1, true ),
    ( "fn:data",                   fn_data,                   1, 1, false ),
// 4.4
    ( "fn:abs",                    fn_abs,                    1, 1, false ),
    ( "fn:ceiling",                fn_ceiling,                1, 1, false ),
    ( "fn:floor",                  fn_floor,                  1, 1, false ),
    ( "fn:round",                  fn_round,                  1, 1, false ),
// 4.5
    ( "fn:number",                 fn_number,                 1, 1, true ),
// 5.2.1
    ( "fn:codepoints-to-string",   fn_codepoints_to_string,   1, 1, false ),
    ( "fn:string-to-codepoints",   fn_string_to_codepoints,   1, 1, false ),
// 5.3
    ( "fn:compare",                fn_compare,                2, 3, false ),
// 5.4
    ( "fn:concat",                 fn_concat,                 2, M, false ),
    ( "fn:string-join",            fn_string_join,            1, 2, false ),
    ( "fn:substring",              fn_substring,              2, 3, false ),
    ( "fn:string-length",          fn_string_length,          1, 1, true ),
    ( "fn:normalize-space",        fn_normalize_space,        1, 1, true ),
    ( "fn:upper-case",             fn_upper_case,             1, 1, false ),
    ( "fn:lower-case",             fn_lower_case,             1, 1, false ),
    ( "fn:translate",              fn_translate,              3, 3, false ),
// 5.5
    ( "fn:contains",               fn_contains,               2, 3, false ),
    ( "fn:starts-with",            fn_starts_with,            2, 3, false ),
    ( "fn:ends-with",              fn_ends_with,              2, 3, false ),
    ( "fn:substring-before",       fn_substring_before,       2, 3, false ),
    ( "fn:substring-after",        fn_substring_after,        2, 3, false ),
// 7.1
    ( "fn:true",                   fn_true,                   0, 0, false ),
    ( "fn:false",                  fn_false,                  0, 0, false ),
// 7.3
    ( "fn:boolean",                fn_boolean,                1, 1, false ),
    ( "fn:not",                    fn_not,                    1, 1, false ),
// 13
    ( "fn:name",                   fn_name,                   1, 1, true ),
    ( "fn:local-name",             fn_local_name,             1, 1, true ),
    ( "fn:namespace-uri",          fn_namespace_uri,          1, 1, true ),
    ( "fn:lang",                   fn_lang,                   1, 2, true ),
    ( "fn:root",                   fn_root,                   1, 1, true ),
// 14.1
    ( "fn:empty",                  fn_empty,                  1, 1, false ),
    ( "fn:exists",                 fn_exists,                 1, 1, false ),
    ( "fn:insert-before",          fn_insert_before,          3, 3, false ),
    ( "fn:remove",                 fn_remove,                 2, 2, false ),
    ( "fn:reverse",                fn_reverse,                1, 1, false ),
    ( "fn:subsequence",            fn_subsequence,            2, 3, false ),
// 14.2
    ( "fn:index-of",               fn_index_of,               2, 3, false ),
// 14.3
    ( "fn:zero-or-one",            fn_zero_or_one,            1, 1, false ),
    ( "fn:one-or-more",            fn_one_or_more,            1, 1, false ),
    ( "fn:exactly-one",            fn_exactly_one,            1, 1, false ),
// 14.4
    ( "fn:count",                  fn_count,                  1, 1, false ),
    ( "fn:avg",                    fn_avg,                    1, 1, false ),
    ( "fn:max",                    fn_max,                    1, 2, false ),
    ( "fn:min",                    fn_min,                    1, 2, false ),
    ( "fn:sum",                    fn_sum,                    1, 2, false ),
    // funcname, func, min_args, max_args, default_is_context_node_set
    // ( "fn:id",                    fn_id,                     1, 2, true ),

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
                eval_env: &mut EvalEnv) -> Result<XSequence, Box<Error>> {

    let mut found = false;
    let mut fn_env: Option<&fn(&Vec<&XSequence>, &mut EvalEnv) -> Result<XSequence, Box<Error>>> = None;
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
    let result = item.get_as_raw_string()?;
    return Ok(new_singleton_string(&result));
}

// ---------------------------------------------------------------------
// 2.4 fn:data
// fn:data($arg as item()*) as xs:anyAtomicType*
//
fn fn_data(args: &Vec<&XSequence>) -> Result<XSequence, Box<Error>> {
    return Ok(args[0].atomize());
}

// ---------------------------------------------------------------------
// 3 Error and diagnostics
//

// ---------------------------------------------------------------------
// 4 Functions and Operators on Numerics
//
// ---------------------------------------------------------------------
// 4.4 Functions on Numeric Values
//        abs
//        ceiling
//        floor
//        round
//        round_half_to_even
//
// ---------------------------------------------------------------------
// 4.4.1 fn:abs
// fn:abs($arg as numeric?) as numeric?
//
fn fn_abs(args: &Vec<&XSequence>) -> Result<XSequence, Box<Error>> {
    return fn_numeric_unary(args,
                |a| { a.abs() },
                |a| { a.abs() },
                |a| { a.abs() });
}

// ---------------------------------------------------------------------
// 4.4.2 fn:ceiling
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
// 4.4.3 fn:floor
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
// 4.4.4 fn:round
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
// 4.5.1 fn:number
// fn:number() as xs:double
// fn:number($arg as xs:anyAtomicType?) as xs:double
//
fn fn_number(args: &Vec<&XSequence>) -> Result<XSequence, Box<Error>> {
    if args[0].is_empty() {
        return Ok(new_singleton_double(f64::NAN));
    }
    let mut result = 0.0;
    if let Ok(arg) = args[0].get_singleton_item() {
        result = arg.get_as_raw_double()?;
    }
    return Ok(new_singleton_double(result));
}

// ---------------------------------------------------------------------
// 5 Functions on Strings
//
// ---------------------------------------------------------------------
// 5.2 Functions to Assemble and Disassemble Strings
//
// ---------------------------------------------------------------------
// 5.2.1 fn:codepoints-to-string
// fn:codepoints-to-string($arg as xs:integer*) as xs:string
//
pub fn fn_codepoints_to_string(args: &Vec<&XSequence>) -> Result<XSequence, Box<Error>> {
    let mut v: Vec<u16> = vec!{};
    for item in args[0].iter() {
        let uni = item.get_as_raw_integer()? as u64;
        if 0x10000 <= uni {                             // 代用対
            let hi = (uni - 0x10000) / 0x0400 + 0xD800;
            let lo = (uni - 0x10000) % 0x0400 + 0xDC00;
            v.push(hi as u16);
            v.push(lo as u16);
        } else {
            v.push(uni as u16);
        }
    }
    match String::from_utf16(&v) {
        Ok(s) => return Ok(new_singleton_string(&s)),
        Err(_) => return Err(dynamic_error!("Code point not valid.")),
    }
}

// ---------------------------------------------------------------------
// 5.2.2 fn:string-to-codepoints
// fn:string-to-codepoints($arg as xs:string?) as xs:integer*
//
pub fn fn_string_to_codepoints(args: &Vec<&XSequence>) -> Result<XSequence, Box<Error>> {
    if args[0].is_empty() {
        return Ok(new_xsequence());
    }
    let arg = args[0].get_singleton_string()?;
    let arg_chars: Vec<char> = arg.chars().collect();
    let mut result = new_xsequence();
    for ch in arg_chars.iter() {
        let mut b = [0; 2];
        ch.encode_utf16(&mut b);

        let hi = b[0] as u64;
        let lo = b[1] as u64;
        if 0xD800 <= hi && hi <= 0xDBFF && 0xDC00 <= lo && lo <= 0xDFFF {
            let uni: u64 = 0x10000 + (hi - 0xD800) * 0x0400 + (lo - 0xDC00);
            result.push(&new_xitem_integer(uni as i64));
        } else {
            result.push(&new_xitem_integer(hi as i64));
        }
    }

    return Ok(result);
}

// ---------------------------------------------------------------------
// 5.3 Comparison of Strings
//
// ---------------------------------------------------------------------
// 5.3.6 fn:compare
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
// 5.4 Functions on String Values
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
// 5.4.1 fn:concat
// fn:concat($arg1 as xs:anyAtomicType?,
//           $arg2 as xs:anyAtomicType?,
//           ... ) as xs:string
//
//                  引数がすべて空シーケンスの場合、空文字列を返す。
//
pub fn fn_concat(args: &Vec<&XSequence>) -> Result<XSequence, Box<Error>> {
    let mut val = String::new();
    for arg in args.iter() {
        if ! arg.is_empty() {
            val += &arg.get_singleton_item()?.get_as_raw_string()?;
        }
    }
    return Ok(new_singleton_string(&val));
}

// ---------------------------------------------------------------------
// 5.4.2 fn:string-join
// fn:string-join($arg1 as xs:string*) as xs:string
// fn:string-join($arg1 as xs:string*, $arg2 as xs:string) as xs:string
//
fn fn_string_join(args: &Vec<&XSequence>) -> Result<XSequence, Box<Error>> {
    let separator = if args.len() < 2 {
            String::new()
        } else {
            args[1].get_singleton_string()?
        };

    let mut result = String::new();
    for (i, s) in args[0].iter().enumerate() {
        if i != 0 {
            result += &separator;
        }
        result += &s.get_as_raw_string()?;
    }
    return Ok(new_singleton_string(&result));
}


// ---------------------------------------------------------------------
// 5.4.3 fn:substring
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

    let starting_loc = args[1].get_singleton_item()?.get_as_raw_double()?;
    let length = if args.len() == 2 {
            f64::INFINITY
        } else {
            args[2].get_singleton_item()?.get_as_raw_double()?
        };
    let (b, e) = subcollection_index_sub(sv.len(), starting_loc, length);

    let mut result = String::new();
    for i in b..e {
        result.push(sv[i]);
    }
    return Ok(new_singleton_string(&result));
}

// ---------------------------------------------------------------------
// 5.4.4 fn:string-length
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
// 5.4.5 fn:normalize-space
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
// 5.4.7 fn:upper-case
// fn:upper-case($arg as xs:string?) as xs:string
//
fn fn_upper_case(args: &Vec<&XSequence>) -> Result<XSequence, Box<Error>> {
    if args[0].is_empty() {
        return Ok(new_singleton_string(&""));
    }

    let arg = args[0].get_singleton_string()?;
    return Ok(new_singleton_string(&arg.to_uppercase()));
}

// ---------------------------------------------------------------------
// 5.4.8 fn:lower-case
// fn:upper-case($arg as xs:string?) as xs:string
//
fn fn_lower_case(args: &Vec<&XSequence>) -> Result<XSequence, Box<Error>> {
    if args[0].is_empty() {
        return Ok(new_singleton_string(&""));
    }

    let arg = args[0].get_singleton_string()?;
    return Ok(new_singleton_string(&arg.to_lowercase()));
}

// ---------------------------------------------------------------------
// 5.4.9 fn:translate
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
// 5.5 Functions Based on Substring Matching
//
// ---------------------------------------------------------------------
// 5.5.1 fn:contains
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
// 5.5.2 fn:starts-with
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
// 5.5.3 fn:ends-with
// fn:start-with($arg1 as xs:string?, $arg2 as xs:string?) as xs:boolean
// fn:start-with($arg1 as xs:string?,
//               $arg2 as xs:string?,
//               $collation as xs:string) as xs:boolean
//
fn fn_ends_with(args: &Vec<&XSequence>) -> Result<XSequence, Box<Error>> {
    let mut arg1 = String::new();
    if ! args[0].is_empty() {
        arg1 = args[0].get_singleton_string()?;
    }

    let mut arg2 = String::new();
    if ! args[1].is_empty() {
        arg2 = args[1].get_singleton_string()?;
    }

    let b = (&arg1).ends_with(&arg2);
    return Ok(new_singleton_boolean(b));
}

// ---------------------------------------------------------------------
// 5.5.4 fn:substring-before
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
// 5.5.5 fn:substring-after
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
// 5.6 String Functions that Use Regular Expressions
//

// ---------------------------------------------------------------------
// 6 Functions that manipulate URIs
//

// ---------------------------------------------------------------------
// 7 Functions and Operators on Boolean Values
//
// ---------------------------------------------------------------------
// 7.1 Boolean Constant Functions
//
// ---------------------------------------------------------------------
// 7.1.1 fn:true
// fn:true() as xs:boolean
//
fn fn_true(_args: &Vec<&XSequence>) -> Result<XSequence, Box<Error>> {
    return Ok(new_singleton_boolean(true));
}

// ---------------------------------------------------------------------
// 7.1.2 fn:false
// fn:false() as xs:boolean
//
fn fn_false(_args: &Vec<&XSequence>) -> Result<XSequence, Box<Error>> {
    return Ok(new_singleton_boolean(false));
}

// ---------------------------------------------------------------------
// 7.3 Functions on Boolean Values
//
// ---------------------------------------------------------------------
// 7.3.1 fn:boolean
// fn:boolean($arg as item()*) as xs:boolean
//      実効ブール値を返す。
//
fn fn_boolean(args: &Vec<&XSequence>) -> Result<XSequence, Box<Error>> {

    let b = args[0].effective_boolean_value()?;
    return Ok(new_singleton_boolean(b));
}

// ---------------------------------------------------------------------
// 7.3.2 fn:not
// fn:not($arg as item()*) as xs:boolean
//      実効ブール値の否定を返す。
//
pub fn fn_not(args: &Vec<&XSequence>) -> Result<XSequence, Box<Error>> {
    let b = args[0].effective_boolean_value()?;
    return Ok(new_singleton_boolean(! b));
}

// ---------------------------------------------------------------------
// 8 Functions and Operators on Durations
// ---------------------------------------------------------------------
// 9 Functions and Operators on Dates and Times
// ---------------------------------------------------------------------
// 10 Functions Related to QNames
// ---------------------------------------------------------------------
// 11 Operators on base64Binary and hexBinary
// ---------------------------------------------------------------------
// 12 Operators on NOTATION

// ---------------------------------------------------------------------
// 13 Functions and Operators on Nodes
//
// ---------------------------------------------------------------------
// 13.1 fn:name
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
// 13.2 fn:local-name
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
// 13.3 fn:namespace-uri
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
// 13.4 fn:lang
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
// 13.5 fn:root
// fn:root() as node()
// fn:root($arg as node()?) as node()?
//
fn fn_root(args: &Vec<&XSequence>) -> Result<XSequence, Box<Error>> {
    if args[0].is_empty() {
        return Ok(new_xsequence());
    }

    if let Ok(node) = args[0].get_singleton_node() {
        let root = node.root();
        return Ok(new_singleton_node(&root));
    } else {
        return Err(dynamic_error!("root(): Item is not a node"));
    }

}

// ---------------------------------------------------------------------
// 14 Functions and Operators on Sequences
//
// ---------------------------------------------------------------------
// 14.1 General Functions and Operators on Sequences
//
// ---------------------------------------------------------------------
// 14.1.1 fn:empty
// fn:empty($arg as item()*) as xs:boolean
//
fn fn_empty(args: &Vec<&XSequence>) -> Result<XSequence, Box<Error>> {
    return Ok(new_singleton_boolean(args[0].len() == 0));
}

// ---------------------------------------------------------------------
// 14.1.2 fn:exists
// fn:exists($arg as item()*) as xs:boolean
//
fn fn_exists(args: &Vec<&XSequence>) -> Result<XSequence, Box<Error>> {
    return Ok(new_singleton_boolean(args[0].len() != 0));
}

// ---------------------------------------------------------------------
// 14.1.5 fn:insert-before
// fn:insert-before($target as item()*,
//                  $position as xs:integer,
//                  $inserts as item()*) as item()*
//
fn fn_insert_before(args: &Vec<&XSequence>) -> Result<XSequence, Box<Error>> {
    let target = args[0];
    let mut position = args[1].get_singleton_integer()? - 1;
    let inserts = args[2];
    if position <= 0 {
        position = 0;
    }
    if target.len() as i64 <= position {
        position = target.len() as i64;
    }
    let position = position as usize;

    let mut result = new_xsequence();
    for i in 0 .. position {
        result.push(target.get_item(i));
    }
    result.append(inserts);
    for i in position .. target.len() {
        result.push(target.get_item(i));
    }
    return Ok(result);
}

// ---------------------------------------------------------------------
// 14.1.6 fn:remove
// fn:remove($target as item()*, $position as xs:integer) as item()*
//
fn fn_remove(args: &Vec<&XSequence>) -> Result<XSequence, Box<Error>> {
    let target = args[0];
    let position = args[1].get_singleton_integer()?;
    let mut result = new_xsequence();
    for n in 0 .. target.len() {
        if n + 1 != position as usize {
            result.push(target.get_item(n));
        }
    }
    return Ok(result);
}

// ---------------------------------------------------------------------
// 14.1.7 fn:reverse
// fn:reverse($arg as item()*) as item()*
//
fn fn_reverse(args: &Vec<&XSequence>) -> Result<XSequence, Box<Error>> {
    let mut arg = args[0].clone();
    arg.reverse();
    return Ok(arg);
}

// ---------------------------------------------------------------------
// 14.1.8 fn:subsequence
// fn:subsequence($sourceSeq as item()*,
//                $startingLoc as xs:double) as item()*
// fn:subsequence($sourceSeq as item()*,
//                $startingLoc as xs:double,
//                $length as xs:double) as item()*
//
fn fn_subsequence(args: &Vec<&XSequence>) -> Result<XSequence, Box<Error>> {

    if args[0].len() == 0 {
        return Ok(new_xsequence());
    }

    let source_sequence = args[0];

    let starting_loc = args[1].get_singleton_item()?.get_as_raw_double()?;
    let length = if args.len() == 2 {
            f64::INFINITY
        } else {
            args[2].get_singleton_item()?.get_as_raw_double()?
        };
    let (b, e) = subcollection_index_sub(
                    source_sequence.len(), starting_loc, length);

    let mut result = new_xsequence();
    for i in b..e {
        result.push(source_sequence.get_item(i));
    }
    return Ok(result);
}

// ---------------------------------------------------------------------
// 14.2 Functions That Compare Values in Sequences
//
// ---------------------------------------------------------------------
// 14.2.2 fn:index-of
// fn:index-of($seqParam as xs:anyAtomicType*,
//             $srchParam as xs:anyAtomicType) as xs:integer*
// fn:index-of($seqParam as xs:anyAtomicType*,
//             $srchParam as xs:anyAtomicType,
//             $collation as xs:string) as xs:integer*
//
fn fn_index_of(args: &Vec<&XSequence>) -> Result<XSequence, Box<Error>> {
    let seq_param = args[0];
    let srch_param = args[1];
    let mut result = new_xsequence();
    for (i, v) in seq_param.iter().enumerate() {
        if value_compare_eq(&new_singleton(v), srch_param)?.get_singleton_boolean()? == true {
            result.push(&new_xitem_integer(usize_to_i64(i + 1)));
        }
    }
    return Ok(result);
}

// ---------------------------------------------------------------------
// 14.3 Functions That Test the Cardinality of Sequences
//
// ---------------------------------------------------------------------
// 14.3.1 fn:zero-or-one
// fn:zero-or-one($arg as item()*) as item()?
//
fn fn_zero_or_one(args: &Vec<&XSequence>) -> Result<XSequence, Box<Error>> {
    if args[0].len() <= 1 {
        return Ok(args[0].clone());
    } else {
        return Err(dynamic_error!("fn:zero-or-one called with a sequence containing more than one item."));
    }
}

// ---------------------------------------------------------------------
// 14.3.2 fn:one-or-more
// fn:one-or-more($arg as item()*) as item()?
//
fn fn_one_or_more(args: &Vec<&XSequence>) -> Result<XSequence, Box<Error>> {
    if 1 <= args[0].len() {
        return Ok(args[0].clone());
    } else {
        return Err(dynamic_error!("fn:one-or-more called with a sequence containing no items."));
    }
}

// ---------------------------------------------------------------------
// 14.3.3 fn:exactly-one
// fn:exactly-one($arg as item()*) as item()?
//
fn fn_exactly_one(args: &Vec<&XSequence>) -> Result<XSequence, Box<Error>> {
    if args[0].len() == 1 {
        return Ok(args[0].clone());
    } else {
        return Err(dynamic_error!("fn:exactly-one called with a sequence containing zero or more than one item."));
    }
}

// ---------------------------------------------------------------------
// 14.4 Aggregate Functions
//
// ---------------------------------------------------------------------
// 14.4.1 fn:count
// fn:count($arg as item()*) as xs:integer
//
fn fn_count(args: &Vec<&XSequence>) -> Result<XSequence, Box<Error>> {
    return Ok(new_singleton_integer(usize_to_i64(args[0].len())));
}

// ---------------------------------------------------------------------
// 14.4.2 fn:avg
// fn:avg($arg as xs:anyAtomicType*) as xs:anyAtomicType?
//
// $argが空シーケンスならば空シーケンスを返す。
// // 加算 (cf. fn:sum) して個数 (cf. fn:count) で除するが、
// // 加算でオーバーフローが生じないようにすること!
//
fn fn_avg(args: &Vec<&XSequence>) -> Result<XSequence, Box<Error>> {
    if args[0].is_empty() {
        return Ok(new_xsequence());
    }

    let sum = fn_sum(args)?;
    let divider = new_xitem_integer(usize_to_i64(args[0].len()));
    let avg = xitem_numeric_divide(&sum.get_item(0), &divider)?;

    return Ok(new_singleton(&avg));
}

// ---------------------------------------------------------------------
// 14.4.3 fn:max
// fn:max($arg as xs:anyAtomicType*) as xs:anyAtomicType?
// fn:max($arg as xs:anyAtomicType*, $collation as string) as xs:anyAtomicType?
//
fn fn_max(args: &Vec<&XSequence>) -> Result<XSequence, Box<Error>> {
    if args[0].is_empty() {
        return Ok(new_xsequence());
    }

    let mut max_item = args[0].get_item(0).clone();
    for item in args[0].iter() {
        let b = value_compare_lt(&new_singleton(&max_item), &new_singleton(item))?;
        if b.effective_boolean_value()? == true {
            max_item = item.clone();
        }
    }

    return Ok(new_singleton(&max_item));
}

// ---------------------------------------------------------------------
// 14.4.4 fn:min
// fn:min($arg as xs:anyAtomicType*) as xs:anyAtomicType?
// fn:min($arg as xs:anyAtomicType*, $collation as string) as xs:anyAtomicType?
//
fn fn_min(args: &Vec<&XSequence>) -> Result<XSequence, Box<Error>> {
    if args[0].is_empty() {
        return Ok(new_xsequence());
    }

    let mut max_item = args[0].get_item(0).clone();
    for item in args[0].iter() {
        let b = value_compare_gt(&new_singleton(&max_item), &new_singleton(item))?;
        if b.effective_boolean_value()? == true {
            max_item = item.clone();
        }
    }

    return Ok(new_singleton(&max_item));
}

// ---------------------------------------------------------------------
// 14.4.5 fn:sum
// fn:sum($arg as xs:anyAtomicType*) as xs:anyAtomicType
// fn:sum($arg as xs:anyAtomicType*,
//        $zero as xs:anyAtomicType?) as xs:anyAtomicType?
//
// $argが空シーケンスのとき: $zeroがあれば$zero、なければ整数0を返す。
//
fn fn_sum(args: &Vec<&XSequence>) -> Result<XSequence, Box<Error>> {
    if args[0].is_empty() {
        if args.len() <= 1 {
            return Ok(new_singleton_integer(0));
        } else {
            return Ok(args[1].clone());
        }
    }

    let mut val = new_xitem_integer(0);
    for n in args[0].iter() {
        if n.is_numeric() {
            val = xitem_numeric_add(&val, &n)?;
        } else {
            let n_double = n.cast_as("double")?;
            val = xitem_numeric_add(&val, &n_double)?;
        }
                        // 必要に応じて型の昇格をしながら加算していく。
    }
    return Ok(new_singleton(&val));
}

// ---------------------------------------------------------------------
// 14.5 Functions on Node Identifiers
//
// ---------------------------------------------------------------------
// 14.6 Functions Giving Access to External Information
//
// ---------------------------------------------------------------------
// 14.7 Parsing and Serializing
//

// ---------------------------------------------------------------------
// 15 Context Functions
//

// ---------------------------------------------------------------------
// 15.1 fn:position
//
fn fn_position(_args: &Vec<&XSequence>, eval_env: &mut EvalEnv) -> Result<XSequence, Box<Error>> {
    return Ok(new_singleton_integer(usize_to_i64(eval_env.get_position())));
}

// ---------------------------------------------------------------------
// 15.2 fn:last
//
fn fn_last(_args: &Vec<&XSequence>, eval_env: &mut EvalEnv) -> Result<XSequence, Box<Error>> {
    return Ok(new_singleton_integer(usize_to_i64(eval_env.get_last())));
}

// ---------------------------------------------------------------------
// 16 Higher-Order Functions
//
// ---------------------------------------------------------------------
// 16.2.1 fn:for-each
// fn:for-each($seq as item()*,
//             $action as function(item()) as item()*) as item()*
//
fn fn_for_each(args: &Vec<&XSequence>, eval_env: &mut EvalEnv) -> Result<XSequence, Box<Error>> {

    let action_xnode = args[1].get_singleton_xnodeptr()?;
    let mut result = new_xsequence();
    for xitem in args[0].iter() {
        let argument_xseq = new_singleton(xitem);
        let result_xseq = call_function(
                    &action_xnode, vec!{argument_xseq}, eval_env)?;
        result.append(&result_xseq);
    }
    return Ok(result);
}

// ---------------------------------------------------------------------
// 16.2.2 fn:filter
// fn:filter($seq as item()*,
//           $f as function(item()) as xs:boolean) as item()*
//
fn fn_filter(args: &Vec<&XSequence>, eval_env: &mut EvalEnv) -> Result<XSequence, Box<Error>> {

    let func_xnode = args[1].get_singleton_xnodeptr()?;
    let mut result = new_xsequence();
    for xitem in args[0].iter() {
        let argument_xseq = new_singleton(xitem);
        let result_xseq = call_function(
                    &func_xnode, vec!{argument_xseq}, eval_env)?;
        if result_xseq.effective_boolean_value()? == true {
            result.push(&xitem);
        }
    }
    return Ok(result);
}



// ---------------------------------------------------------------------
// 17 Maps and Arrays
// ---------------------------------------------------------------------
// 18 Constructor Functions
// ---------------------------------------------------------------------
// 19 Casting
// ---------------------------------------------------------------------
//

// =====================================================================
// 補助函数
//
// ---------------------------------------------------------------------
// ある長さの順序つき集合 (C; 文字列、シーケンスなど) の部分集合を
// 取得するために、開始位置 b と終了位置 e を求める。
// s (starting_loc、1起点の値) と l (length) はf64型で、NaNやInfにもなりうる。
// Cの要素 (番号 i := [b, e)、0起点の値) から成る部分集合を
// 取得すればよいよう、bとe (usize型) を求めて返す。
// 空集合を取得するべき場合は、b = 0、e = 0 を返す。
//
//                  7.4.3 fn:substring
//                  15.1.10 fn:subsequence
//
fn subcollection_index_sub(source_length: usize,
                           starting_loc: f64, length: f64) -> (usize, usize) {

    if starting_loc.is_nan() || starting_loc.is_infinite() {
        return (0, 0);
    }
    let beg_pos = round_x(starting_loc) as i64;     // 有限値
    let mut b = beg_pos - 1;                        // 0起点の値に補正
    if b < 0 {
        b = 0;
    }
    if source_length as i64 <= b {
        b = source_length as i64;
    }

    let mut e: i64;
    if length.is_infinite() && length.is_sign_positive() {
        e = source_length as i64;
    } else {
        if length.is_nan() || length.is_sign_negative() {
            return (0, 0);
        }
        let len_str = if length.is_infinite() {
                source_length as i64
            } else {
                round_x(length) as i64                 // 非負の有限値
            };
        e = beg_pos + len_str - 1;
        if e < b {
            e = b;
        }
        if source_length as i64 <= e {
            e = source_length as i64;
        }
    }

    return (b as usize, e as usize);

}

// =====================================================================
//
#[cfg(test)]
mod test {
//    use super::*;

    use xpath_impl::helpers::compress_spaces;
    use xpath_impl::helpers::subtest_xpath;
    use xpath_impl::helpers::subtest_eval_xpath;

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
    // 2.4 fn:data
    //
    #[test]
    fn test_fn_data() {
        let xml = compress_spaces(r#"
<a base="base">
    Data
</a>
        "#);
        subtest_eval_xpath("fn_data", &xml, &[
            ( r#"data((/a, 37))"#, r#"("Data", 37)"# ),
        ]);
    }

    // -----------------------------------------------------------------
    // 4.4.1 fn:abs
    //
    #[test]
    fn test_fn_abs() {
        let xml = compress_spaces(r#"
<a base="base">
</a>
        "#);
        subtest_eval_xpath("fn_abs", &xml, &[
            ( "abs(10.5)", "(10.5)" ),
            ( "abs(-10.5)", "(10.5)" ),
            ( "abs(-0e0)", "(0e0)" ),
            ( "abs(-1 div 0e0)", "(+Infinity)" ),
        ]);
    }

    // -----------------------------------------------------------------
    // 4.4.2 fn:ceiling
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
    // 4.4.3 fn:floor
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
    // 4.4.4 fn:round
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
    // 5.2.1 fn:codepoints-to-string
    //
    #[test]
    fn test_fn_codepoints_to_string() {
        let xml = compress_spaces(r#"
<root base="base">
</root>
        "#);
        subtest_eval_xpath("fn_codepoints_to_string", &xml, &[
            ( r#"codepoints-to-string((84, 104, 233, 114, 232, 115, 101))"#, r#"("Thérèse")"# ),
            ( r#"codepoints-to-string((131072, 131073, 131074))"#, r#"("𠀀𠀁𠀂")"# ),
        ]);
    }

    // -----------------------------------------------------------------
    // 5.2.2 fn:string-to-codepoints
    //
    #[test]
    fn test_fn_string_to_codepoints() {
        let xml = compress_spaces(r#"
<root base="base">
</root>
        "#);
        subtest_eval_xpath("fn_string_to_codepoints", &xml, &[
            ( r#"string-to-codepoints("Thérèse")"#, r#"(84, 104, 233, 114, 232, 115, 101)"# ),
            ( r#"string-to-codepoints("𠀀𠀁𠀂")"#, r#"(131072, 131073, 131074)"# ),
                                            // 0x20000 = 131072
        ]);
    }

    // -----------------------------------------------------------------
    // 5.3.6 fn:compare
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
    // 5.4.1 fn:concat
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
    // 5.4.2 fn:string-join
    //
    #[test]
    fn test_fn_string_join() {
        let xml = compress_spaces(r#"
<doc>
    <chap>
        <section base="base">
        </section>
    </chap>
</doc>
        "#);
        subtest_eval_xpath("fn_string_join", &xml, &[
            ( r#"string-join(('A', 'B', 'C'), 'x')"#, r#"("AxBxC")"# ),
            ( r#"string-join(for $n in ancestor-or-self::* return name($n), '/')"#, r#"("doc/chap/section")"# ),
        ]);
    }

    // -----------------------------------------------------------------
    // 5.4.3 fn:substring
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
    // 5.4.4 fn:string-length
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
    // 5.4.5 fn:normalize-space
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
    // 5.4.7 fn:upper-case
    //
    #[test]
    fn test_fn_upper_case() {
        let xml = compress_spaces(r#"
<a base="base">
</a>
        "#);
        subtest_eval_xpath("fn_upper_case", &xml, &[
            ( r#"upper-case('AbCdE')"#, r#"("ABCDE")"# ),
            ( r#"upper-case('ΣЯσя')"#, r#"("ΣЯΣЯ")"# ),
        ]);
    }

    // -----------------------------------------------------------------
    // 5.4.8 fn:lower-case
    //
    #[test]
    fn test_fn_lower_case() {
        let xml = compress_spaces(r#"
<a base="base">
</a>
        "#);
        subtest_eval_xpath("fn_lower_case", &xml, &[
            ( r#"lower-case('AbCdE')"#, r#"("abcde")"# ),
            ( r#"lower-case('ΣЯσя')"#, r#"("σяσя")"# ),
        ]);
    }

    // -----------------------------------------------------------------
    // 5.4.9 fn:translate
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
    // 5.5.1 fn:contains
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
    // 5.5.2 fn:starts-with
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
    // 5.5.3 fn:ends-with
    //
    #[test]
    fn test_fn_ends_with() {
        let xml = compress_spaces(r#"
<a base="base">
</a>
        "#);
        subtest_eval_xpath("fn_ends_with", &xml, &[
            ( r#"ends-with("かきくけこ", "けこ")"#, "(true)" ),
            ( r#"ends-with("かきくけこ", "てと")"#, "(false)" ),
            ( r#"ends-with("", "てと")"#, "(false)" ),
            ( r#"ends-with("かきくけこ", "")"#, "(true)" ),
            ( r#"ends-with("", "")"#, "(true)" ),
        ]);
    }

    // -----------------------------------------------------------------
    // 5.5.4 fn:substring-before
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
    // 5.5.5 fn:substring-after
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
    // 13.3 fn:namespace-uri
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
    // 13.4 fn:lang
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
    // 13.5 fn:root
    //
    #[test]
    fn test_fn_root() {
        let xml = compress_spaces(r#"
<?xml version='1.0' encoding='UTF-8'?>
<root>
    <para base="base"/>
</root>
        "#);
        subtest_eval_xpath("fn_root", &xml, &[
            ( "root()", "((DocumentRoot))" ),
            ( "root(/root/para)", "((DocumentRoot))" ),
            ( "root(/root/empty)", "()" ),
            ( "root(45)", "Dynamic Error" ),
        ]);
    }

    // -----------------------------------------------------------------
    // 14.1.1 fn:empty
    //
    #[test]
    fn test_fn_empty() {
        let xml = compress_spaces(r#"
<root base="base">
</root>
        "#);
        subtest_eval_xpath("fn_empty", &xml, &[
            ( "empty(())", "(true)" ),
            ( r#"empty(("ABC"))"#, "(false)" ),
        ]);
    }

    // -----------------------------------------------------------------
    // 14.1.2 fn:exists
    //
    #[test]
    fn test_fn_exists() {
        let xml = compress_spaces(r#"
<root base="base">
</root>
        "#);
        subtest_eval_xpath("fn_exists", &xml, &[
            ( "exists(())", "(false)" ),
            ( r#"exists(("ABC"))"#, "(true)" ),
        ]);
    }

    // -----------------------------------------------------------------
    // 14.1.5 fn:insert-before
    //
    #[test]
    fn test_fn_insert_before() {
        let xml = compress_spaces(r#"
<root base="base">
</root>
        "#);
        subtest_eval_xpath("fn_insert_before", &xml, &[
            ( "insert-before((1, 2, 3), 0, 99)", "(99, 1, 2, 3)" ),
            ( "insert-before((1, 2, 3), 1, 99)", "(99, 1, 2, 3)" ),
            ( "insert-before((1, 2, 3), 2, 99)", "(1, 99, 2, 3)" ),
            ( "insert-before((1, 2, 3), 3, 99)", "(1, 2, 99, 3)" ),
            ( "insert-before((1, 2, 3), 4, 99)", "(1, 2, 3, 99)" ),
            ( "insert-before((1, 2, 3), 2, (98, 99))", "(1, 98, 99, 2, 3)" ),
        ]);
    }

    // -----------------------------------------------------------------
    // 14.1.6 fn:remove
    //
    #[test]
    fn test_fn_remove() {
        let xml = compress_spaces(r#"
<root base="base">
</root>
        "#);
        subtest_eval_xpath("fn_remove", &xml, &[
            ( r#"remove(("A", "B", "C"), 0)"#, r#"("A", "B", "C")"# ),
            ( r#"remove(("A", "B", "C"), 2)"#, r#"("A", "C")"# ),
            ( r#"remove((), 3)"#, r#"()"# ),
        ]);
    }

    // -----------------------------------------------------------------
    // 14.1.7 fn:reverse
    //
    #[test]
    fn test_fn_reverse() {
        let xml = compress_spaces(r#"
<root base="base">
</root>
        "#);
        subtest_eval_xpath("fn_reverse", &xml, &[
            ( r#"reverse(("A", "B", "C"))"#, r#"("C", "B", "A")"# ),
            ( r#"reverse(())"#, r#"()"# ),
        ]);
    }

    // -----------------------------------------------------------------
    // 14.1.8 fn:subsequence
    //
    #[test]
    fn test_fn_subsequence() {
        let xml = compress_spaces(r#"
<root base="base">
</root>
        "#);
        subtest_eval_xpath("fn_subsequence", &xml, &[
            ( "subsequence((), 2, 2)", "()" ),
            ( "subsequence((1, 2, 3, 4), 2)", "(2, 3, 4)" ),
            ( "subsequence((1, 2, 3, 4), 2, 2)", "(2, 3)" ),
            ( "subsequence((1, 2, 3, 4), -2, 5)", "(1, 2)" ),
            ( "subsequence((1, 2, 3, 4), -42, 1 div 0e0)", "(1, 2, 3, 4)" ),
        ]);
    }

    // -----------------------------------------------------------------
    // 14.2.2 fn:index-of
    //
    #[test]
    fn test_fn_index_of() {
        let xml = compress_spaces(r#"
<root base="base">
</root>
        "#);
        subtest_eval_xpath("fn_index_of", &xml, &[
            ( "index-of((10, 20, 30, 40), 25)", "()" ),
            ( "index-of((10, 20, 30, 30, 20, 10), 20)", "(2, 5)" ),
            ( "index-of(('a', 'sport', 'and', 'a', 'pastime'), 'a')", "(1, 4)" ),
        ]);
    }

    // -----------------------------------------------------------------
    // 14.3.1 fn:zero-or-one
    //
    #[test]
    fn test_fn_zero_or_one() {
        let xml = compress_spaces(r#"
<root base="base">
</root>
        "#);
        subtest_eval_xpath("fn_zero_or_one", &xml, &[
            ( "zero-or-one(())", "()" ),
            ( "zero-or-one((5))", "(5)" ),
            ( "zero-or-one((5, 8))", "Dynamic Error" ),
        ]);
    }

    // -----------------------------------------------------------------
    // 14.3.2 fn:one-or-more
    //
    #[test]
    fn test_fn_one_or_more() {
        let xml = compress_spaces(r#"
<root base="base">
</root>
        "#);
        subtest_eval_xpath("fn_one_or_more", &xml, &[
            ( "one-or-more(())", "Dynamic Error" ),
            ( "one-or-more((5))", "(5)" ),
            ( "one-or-more((5, 8))", "(5, 8)" ),
        ]);
    }

    // -----------------------------------------------------------------
    // 14.3.3 fn:exactly-one
    //
    #[test]
    fn test_fn_exactly_one() {
        let xml = compress_spaces(r#"
<root base="base">
</root>
        "#);
        subtest_eval_xpath("fn_exactly_one", &xml, &[
            ( "exactly-one(())", "Dynamic Error" ),
            ( "exactly-one((5))", "(5)" ),
            ( "exactly-one((5, 8))", "Dynamic Error" ),
        ]);
    }

    // -----------------------------------------------------------------
    // 14.4.2 fn:avg
    //
    #[test]
    fn test_fn_avg() {
        let xml = compress_spaces(r#"
<root base="base">
</root>
        "#);
        subtest_eval_xpath("fn_avg", &xml, &[
            ( "avg(())", "()" ),
            ( "avg((3, 4, 5))", "(4.0)" ),
            ( "avg((1e0 div 0e0, 1e0 div 0e0))", "(+Infinity)" ),
            ( "avg((1e0 div 0e0, -1e0 div 0e0))", "(NaN)" ),
        ]);
    }

    // -----------------------------------------------------------------
    // 14.4.3 fn:max
    //
    #[test]
    fn test_fn_max() {
        let xml = compress_spaces(r#"
<root base="base">
</root>
        "#);
        subtest_eval_xpath("fn_max", &xml, &[
            ( "max(())", "()" ),
            ( "max((3, 4, 5))", "(5)" ),
            ( r#"max(("a", "b", "c"))"#, r#"("c")"# ),
            ( r#"max((3, 4, "zero"))"#, "Type Error" ),
        ]);
    }

    // -----------------------------------------------------------------
    // 14.4.4 fn:min
    //
    #[test]
    fn test_fn_min() {
        let xml = compress_spaces(r#"
<root base="base">
</root>
        "#);
        subtest_eval_xpath("fn_min", &xml, &[
            ( "min(())", "()" ),
            ( "min((3, 4, 5))", "(3)" ),
            ( r#"min(("a", "b", "c"))"#, r#"("a")"# ),
            ( r#"min((3, 4, "zero"))"#, "Type Error" ),
        ]);
    }

    // -----------------------------------------------------------------
    // 14.4.5 fn:sum
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
            ( "sum((), (99))", "(99)" ),
            ( r#"sum(("1", "2", "3"))"#, "(6e0)" ),
        ]);
    }

    // -----------------------------------------------------------------
    // 15.1 fn:position
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
    // 15.2 fn:last
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

    // -----------------------------------------------------------------
    // 16.2.1 fn:for-each
    //
    #[test]
    fn test_fn_for_each() {
        let xml = compress_spaces(r#"
<root>
</root>
        "#);
        subtest_eval_xpath("fn_for_each", &xml, &[
            ( "for-each(1 to 4, function($x as integer) { $x * $x })", "(1, 4, 9, 16)" ),
            ( r#"for-each(("john", "jane"), fn:string-to-codepoints#1)"#,
                        "(106, 111, 104, 110, 106, 97, 110, 101)" ),
        ]);
    }

    // -----------------------------------------------------------------
    // 16.2.2 fn:filter
    //
    #[test]
    fn test_fn_filter() {
        let xml = compress_spaces(r#"
<root>
</root>
        "#);
        subtest_eval_xpath("fn_filter", &xml, &[
            ( "filter(1 to 10, function($a) { $a mod 2 = 0 })", "(2, 4, 6, 8, 10)" ),
        ]);
    }
}
