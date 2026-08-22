﻿﻿﻿﻿﻿﻿﻿import ctypes
import os
import tempfile

class RustBedrockConverter:
    """Rust-Python桥接类，用于调用Rust实现的基岩版转换功能"""
    
    def __init__(self, output_dir: str = "converted_packs"):
        # 加载Rust DLL (优先使用release版本)
        dll_path = os.path.join(os.path.dirname(__file__), "rust", "target", "release", "kpt_bedrock_converter.dll")
        if not os.path.exists(dll_path):
            # 如果release版本不存在，回退到debug版本
            dll_path = os.path.join(os.path.dirname(__file__), "rust", "target", "debug", "kpt_bedrock_converter.dll")
            if not os.path.exists(dll_path):
                raise FileNotFoundError(f"Rust DLL not found at {dll_path}")
        
        self.dll = ctypes.CDLL(dll_path)
        
        # 配置函数接口
        self._configure_functions()
        
        # 设置目录
        self.temp_dir = tempfile.mkdtemp(prefix="bedrock_conv_")
        self.output_dir = output_dir
        
        # 创建输出目录
        os.makedirs(output_dir, exist_ok=True)
    
    def _configure_functions(self):
        """配置Rust FFI函数接口"""
        # configure_functions 的配置设置
        self.dll.convert_java_to_bedrock_ffi.restype = ctypes.c_void_p
        self.dll.convert_java_to_bedrock_ffi.argtypes = [
            ctypes.c_char_p,
            ctypes.c_char_p,
            ctypes.c_char_p
        ]
        
        self.dll.free_conversion_result.restype = None
        self.dll.free_conversion_result.argtypes = [ctypes.c_void_p]
    
    def convert_to_bedrock(self, java_pack_path: str) -> tuple[bool, str]:
        """将Java版材质包转换为基岩版
        
        Args:
            java_pack_path: Java版材质包的路径
            
        Returns:
            tuple[bool, str]: (是否成功, 结果路径或错误信息)
        """
        try:
            # 获取输入文件的目录作为输出目录
            input_dir = os.path.dirname(java_pack_path)
            
            # 转换路径为字节字符串
            java_pack_path_bytes = java_pack_path.encode('utf-8')
            temp_dir_bytes = self.temp_dir.encode('utf-8')
            output_dir_bytes = input_dir.encode('utf-8')
            
            # 调用Rust函数
            result_ptr = self.dll.convert_java_to_bedrock_ffi(
                java_pack_path_bytes,
                temp_dir_bytes,
                output_dir_bytes
            )
            
            if result_ptr is None:
                return False, "调用Rust函数失败"
            
            # 定义结体类型
            class ConversionResultFFI(ctypes.Structure):
                _fields_ = [
                    ("success", ctypes.c_int),
                    ("input_file", ctypes.c_char_p),
                    ("output_file", ctypes.c_char_p),
                    ("error_message", ctypes.c_char_p),
                    ("conversion_time_ms", ctypes.c_uint64),
                    ("warnings_count", ctypes.c_int),
                    ("warnings", ctypes.POINTER(ctypes.c_char_p))
                ]
            
            # 直接将指针转换为具体的结体类型
            result = ctypes.cast(result_ptr, ctypes.POINTER(ConversionResultFFI)).contents
            
            # 处理结果
            success = bool(result.success)
            output_file = result.output_file.decode('utf-8') if result.output_file else ""
            error_message = result.error_message.decode('utf-8') if result.error_message else ""
            
            # 释放内存
            self.dll.free_conversion_result(result_ptr)
            
            if success:
                return True, output_file
            else:
                return False, error_message or "转换失败，原因未知"
                
        except Exception as e:
            return False, f"转换过程中发生错误: {str(e)}"
    
    def __del__(self):
        """清理临时目录"""
        try:
            import shutil
            if hasattr(self, 'temp_dir') and os.path.exists(self.temp_dir):
                shutil.rmtree(self.temp_dir, ignore_errors=True)
        except Exception:
            pass
