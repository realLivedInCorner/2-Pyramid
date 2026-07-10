import os
import traceback

# 注意：log函数需要从原文件中提取
def rename_mcpatcher_to_optifine(temp_dir):
    try:
        # 重命名 mcpatcher 文件夹为 optifine 文件夹
        mcpatcher_path = os.path.join(temp_dir, 'assets', 'minecraft', 'mcpatcher')
        optifine_path = os.path.join(temp_dir, 'assets', 'minecraft', 'optifine')

        if os.path.exists(mcpatcher_path):
            if not os.path.exists(optifine_path):
                os.rename(mcpatcher_path, optifine_path)
                log(f"已将 'mcpatcher' 重命名为 'optifine' 在 {os.path.join(temp_dir, 'assets', 'minecraft')}")
            else:
                log(f"跳过重命名，因为 'optifine' 文件夹已存在于 {os.path.join(temp_dir, 'assets', 'minecraft')}")
        else:
            log(f"未找到 'mcpatcher' 文件夹在 {os.path.join(temp_dir, 'assets', 'minecraft')}")
    except Exception as e:
        log(f"Error processing 'mcpatcher': {e}")
        traceback.print_exc()