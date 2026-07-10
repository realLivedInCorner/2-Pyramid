import os
import traceback

# 注意：此函数依赖于外部的log函数和多个process_*函数

def cut_gui(temp_dir):
    try:
        gui_path = os.path.join(temp_dir, 'assets', 'minecraft', 'textures', 'gui')
        process_icons_in_dir(temp_dir)
        process_widgets_in_dir(temp_dir)
        process_tabs_in_dir(temp_dir)
        process_resource_packs_in_dir(temp_dir)
        process_server_selection_in_dir(temp_dir)
        process_title_in_dir(temp_dir)
        process1_anvil_image(temp_dir)
        process1_beacon_image(temp_dir)
        process1_blast_furnace_image(temp_dir)
        process1_brewing_stand_image(temp_dir)
        process1_cartography_table_image(temp_dir)
        process1_enchanting_table_image(temp_dir)
        process1_furnace_image(temp_dir)
        process1_smoker_image(temp_dir)
        process1_grindstone_image(temp_dir)
        process1_horse_image(temp_dir)
        process1_inventory_image(temp_dir)
        process1_loom_image(temp_dir)
        process1_stonecutter_image(temp_dir)
        process1_smithing_image(temp_dir)
        process1_villager2_image(temp_dir)
        process1_slider_image(gui_path)
    except Exception as e:
        log(f"Error processing cut gui image in '{temp_dir}': {e}")
        traceback.print_exc()