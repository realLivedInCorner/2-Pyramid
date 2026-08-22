//! Foreign Function Interface for Python-Rust integration
//! Python和Rust桥接接口

use crate::{BedrockConverter, ConversionResult, ConversionStats};
use crate::utils::Utils;
use std::path::{Path, PathBuf};
use std::ffi::{CStr, CString, c_char};
use std::os::raw::c_int;
use std::slice;

/// 转换结果的结构体（FFI版本）
#[repr(C)]
pub struct ConversionResultFFI {
    pub success: c_int,
    pub input_file: *const c_char,
    pub output_file: *const c_char,
    pub error_message: *const c_char,
    pub conversion_time_ms: u64,
    pub warnings_count: c_int,
    pub warnings: *const *const c_char,
}

/// 创建一个空的转换结果FFI结构体
#[no_mangle]
pub extern "C" fn create_empty_conversion_result() -> *mut ConversionResultFFI {
    let result = Box::new(ConversionResultFFI {
        success: 0,
        input_file: std::ptr::null(),
        output_file: std::ptr::null(),
        error_message: std::ptr::null(),
        conversion_time_ms: 0,
        warnings_count: 0,
        warnings: std::ptr::null(),
    });
    Box::into_raw(result)
}

/// 释放转换结果FFI结构体
#[no_mangle]
pub extern "C" fn free_conversion_result(result: *mut ConversionResultFFI) {
    if !result.is_null() {
        let boxed_result = unsafe { Box::from_raw(result) };
        
        // 释放字符串
        if !boxed_result.input_file.is_null() {
            unsafe { CString::from_raw(boxed_result.input_file as *mut c_char) };
        }
        if !boxed_result.output_file.is_null() {
            unsafe { CString::from_raw(boxed_result.output_file as *mut c_char) };
        }
        if !boxed_result.error_message.is_null() {
            unsafe { CString::from_raw(boxed_result.error_message as *mut c_char) };
        }
        
        // 释放警告数组
        if !boxed_result.warnings.is_null() {
            let warnings_slice = unsafe { slice::from_raw_parts(boxed_result.warnings, boxed_result.warnings_count as usize) };
            for warning_ptr in warnings_slice {
                if !warning_ptr.is_null() {
                    unsafe { CString::from_raw(*warning_ptr as *mut c_char) };
                }
            }
            unsafe { 
                let warnings_ptr = boxed_result.warnings as *mut libc::c_void;
                libc::free(warnings_ptr); 
            };
        }
        
        drop(boxed_result);
    }
}

/// 将Rust ConversionResult转换为FFI版本
fn convert_result_to_ffi(result: &ConversionResult) -> *mut ConversionResultFFI {
    let input_file_cstr: CString = CString::new(result.input_file.to_string_lossy().as_ref()).unwrap_or_else(|_| CString::new("").unwrap());
    let output_file_cstr: CString = if let Some(ref output) = result.output_file {
        CString::new(output.to_string_lossy().as_ref()).unwrap_or_else(|_| CString::new("").unwrap())
    } else {
        CString::new("").unwrap()
    };
    let error_message_cstr: CString = if let Some(ref error) = result.error_message {
        CString::new(error.as_str()).unwrap_or_else(|_| CString::new("").unwrap())
    } else {
        CString::new("").unwrap()
    };
    
    // 处理警告数组 - 使用更简单的内存管理方式
    let warnings_count: c_int = result.warnings.len() as c_int;
    let warnings_ptr: *const *const c_char;
    
    if warnings_count > 0 {
        // 分配足够的内存来存储警告字符串指针
        let warnings_array_ptr = unsafe {
            libc::malloc(std::mem::size_of::<*const c_char>() * warnings_count as usize) as *mut *const c_char
        };
        
        if warnings_array_ptr.is_null() {
            // 内存分配失败，返回空结果
            return create_empty_conversion_result();
        }
        
        // 将警告字符串转换为CString并存储指针
        for (i, warning) in result.warnings.iter().enumerate() {
            if let Ok(warning_cstr) = CString::new(warning.as_str()) {
                // 转换为原始指针并存储
                let warning_ptr = warning_cstr.into_raw();
                unsafe {
                    *warnings_array_ptr.offset(i as isize) = warning_ptr;
                }
            }
        }
        
        warnings_ptr = warnings_array_ptr as *const *const c_char;
    } else {
        warnings_ptr = std::ptr::null();
    }
    
    let result_ffi: Box<ConversionResultFFI> = Box::new(ConversionResultFFI {
        success: if result.success { 1 } else { 0 },
        input_file: input_file_cstr.into_raw(),
        output_file: output_file_cstr.into_raw(),
        error_message: error_message_cstr.into_raw(),
        conversion_time_ms: result.conversion_time_ms,
        warnings_count,
        warnings: warnings_ptr,
    });
    
    Box::into_raw(result_ffi)
}

/// 转换Java版材质包为基岩版格式（FFI接口）
#[no_mangle]
pub extern "C" fn convert_java_to_bedrock_ffi(
    java_pack_path: *const c_char,
    temp_dir: *const c_char,
    output_dir: *const c_char
) -> *mut ConversionResultFFI {
    // 检查空指针
    if java_pack_path.is_null() || temp_dir.is_null() || output_dir.is_null() {
        return create_empty_conversion_result();
    }
    
    let java_pack_path_str: &str = unsafe {
        match CStr::from_ptr(java_pack_path).to_str() {
            Ok(s) => s,
            Err(_) => return create_empty_conversion_result(),
        }
    };
    
    let temp_dir_str: &str = unsafe {
        match CStr::from_ptr(temp_dir).to_str() {
            Ok(s) => s,
            Err(_) => return create_empty_conversion_result(),
        }
    };
    
    let output_dir_str: &str = unsafe {
        match CStr::from_ptr(output_dir).to_str() {
            Ok(s) => s,
            Err(_) => return create_empty_conversion_result(),
        }
    };
    
    let temp_path: PathBuf = PathBuf::from(temp_dir_str);
    let output_path: PathBuf = PathBuf::from(output_dir_str);
    
    let converter_result: Result<BedrockConverter, String> = BedrockConverter::new(temp_path, output_path);
    match converter_result {
        Ok(converter) => {
            let java_pack_buf: PathBuf = PathBuf::from(java_pack_path_str);
            
            // 执行转换
            let result = converter.convert_pack(&java_pack_buf);
            
            // 转换为FFI格式
            convert_result_to_ffi(&result)
        }
        Err(_) => create_empty_conversion_result(),
    }
}

/// 批量转换多个材质包（FFI接口）
#[no_mangle]
pub extern "C" fn convert_multiple_packs_ffi(
    java_pack_paths: *const *const c_char,
    paths_count: c_int,
    temp_dir: *const c_char,
    output_dir: *const c_char
) -> *mut ConversionResultFFI {
    if paths_count <= 0 || java_pack_paths.is_null() {
        return create_empty_conversion_result();
    }
    
    // 检查空指针
    if temp_dir.is_null() || output_dir.is_null() {
        return create_empty_conversion_result();
    }
    
    let temp_dir_str: &str = unsafe {
        match CStr::from_ptr(temp_dir).to_str() {
            Ok(s) => s,
            Err(_) => return create_empty_conversion_result(),
        }
    };
    
    let output_dir_str: &str = unsafe {
        match CStr::from_ptr(output_dir).to_str() {
            Ok(s) => s,
            Err(_) => return create_empty_conversion_result(),
        }
    };
    
    let temp_path: PathBuf = PathBuf::from(temp_dir_str);
    let output_path: PathBuf = PathBuf::from(output_dir_str);
    
    let converter: BedrockConverter = match BedrockConverter::new(temp_path, output_path) {
        Ok(converter) => converter,
        Err(_) => return create_empty_conversion_result(),
    };
    
    // 收集所有路径
    let mut paths: Vec<PathBuf> = Vec::new();
    for i in 0..paths_count {
        let path_ptr = unsafe { *java_pack_paths.offset(i as isize) };
        if !path_ptr.is_null() {
            match unsafe { CStr::from_ptr(path_ptr).to_str() } {
                Ok(s) => paths.push(PathBuf::from(s)),
                Err(_) => continue,
            }
        }
    }
    
    if paths.is_empty() {
        return create_empty_conversion_result();
    }
    let path_refs: Vec<&Path> = paths.iter().map(|p: &PathBuf| p.as_path()).collect();
    let results: Vec<ConversionResult> = converter.convert_multiple_packs(&path_refs);
    
    // 返回第一个结果作为示例（实际应用中可以返回更复杂的结果）
    if let Some(first_result) = results.first() {
        convert_result_to_ffi(first_result)
    } else {
        create_empty_conversion_result()
    }
}

/// 获取转换器统计信息（FFI接口）
#[no_mangle]
pub extern "C" fn get_converter_stats_ffi(
    temp_dir: *const c_char,
    output_dir: *const c_char
) -> *mut c_char {
    if temp_dir.is_null() || output_dir.is_null() {
        return std::ptr::null_mut();
    }
    
    let temp_dir_str: &str = unsafe {
        match CStr::from_ptr(temp_dir).to_str() {
            Ok(s) => s,
            Err(_) => return std::ptr::null_mut(),
        }
    };
    
    let output_dir_str: &str = unsafe {
        match CStr::from_ptr(output_dir).to_str() {
            Ok(s) => s,
            Err(_) => return std::ptr::null_mut(),
        }
    };
    
    let temp_path: PathBuf = PathBuf::from(temp_dir_str);
    let output_path: PathBuf = PathBuf::from(output_dir_str);
    
    match BedrockConverter::new(temp_path, output_path).map_err(|_| String::new()) {
        Ok(converter) => {
            let stats = converter.get_converter_stats();
            let stats_json: String = format!(r#"{{"total_files": {},"total_size_bytes": {},"directories_count": {},"largest_file": {:?},"largest_file_size": {}}}"#,
                stats.total_files,
                stats.total_size_bytes,
                stats.directories_count,
                stats.largest_file,
                stats.largest_file_size
            );
            
            let cstr: CString = CString::new(stats_json.as_str()).unwrap_or_else(|_| CString::new("{}").unwrap());
            cstr.into_raw()
        }
        Err(_) => std::ptr::null_mut(),
    }
}

/// 释放字符串
#[no_mangle]
pub extern "C" fn free_string(s: *mut c_char) {
    if !s.is_null() {
        unsafe { CString::from_raw(s) };
    }
}

/// 初始化日志系统
#[no_mangle]
pub extern "C" fn init_logging_ffi() {
    Utils::init_logging();
}