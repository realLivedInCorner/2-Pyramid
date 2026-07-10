def scaled_point(x, y, scale_factor):
    """
    根据scale_factor缩放单点坐标
    参数:
        x, y: 点坐标
        scale_factor: 缩放因子
    返回:
        缩放后的坐标元组 (x*scale_factor, y*scale_factor)
    """
    return (x * scale_factor, y * scale_factor)