def scaled_coords(x1, y1, x2, y2, scale_factor):
    """
    根据scale_factor缩放坐标
    参数:
        x1, y1: 左上角坐标
        x2, y2: 右下角坐标
        scale_factor: 缩放因子
    返回:
        缩放后的坐标元组 (x1*scale_factor, y1*scale_factor, x2*scale_factor, y2*scale_factor)
    """
    return (x1 * scale_factor, y1 * scale_factor, x2 * scale_factor, y2 * scale_factor)