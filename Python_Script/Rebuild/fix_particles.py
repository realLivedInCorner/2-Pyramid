import os
import traceback

# 注意：log函数、split_particles_image函数需要从原文件中提取
def fix_particles(temp_dir):
    """
    处理 temp_dir 中的 assets/minecraft/textures/particle/particles.png 文件，
    """
    try:
        # 定义 particles.png 的路径
        particles_path = os.path.join(temp_dir, "assets", "minecraft", "textures", "particle", "particles.png")
        
        log(f"正在处理 particles.png 在: {particles_path}")
        
        if os.path.exists(particles_path):
            split_particles_image(particles_path)
            log(f"已分割 'particles.png' 在 {particles_path}")
        else:
            log(f"未找到 {temp_dir} 中的 'particles.png'")
    
    except Exception as e:
        log(f"处理 'particles.png' 时出错: {e}")
        traceback.print_exc()