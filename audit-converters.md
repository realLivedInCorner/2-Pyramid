# Converter rs↔py Mapping 矩阵

- rs 端 converter 文件: 71
- rs 端 pub fn 总数: 88
- py 端 def 总数(pack.py): 144
- 两边都有的: 54
- rs 有 py 缺(可能 py 端被废弃): 34
- py 有 rs 缺(可能需要补): 90

## ⚠️  行数差异 Top 30 (按 |rs-py| 差值排序)

| converter | rs 文件 | rs 行数 | py 行号 | py 行数 | 差值 | 倍数 |
|---|---|---:|---:|---:|---:|---:|
| fix_smithing2_villager2_ui | fix_smithing2_villager2_ui.rs | 13 | 8231 | 279 | -266 | 0.05x |
| fix_alpha_layers_in_textures | fix_alpha_layers_in_textures.rs | 50 | 5957 | 226 | -176 | 0.22x |
| fix_brewing_stand_ui | fix_brewing_stand_ui.rs | 5 | 6655 | 80 | -75 | 0.06x |
| fix_tabs | fix_tabs.rs | 130 | 8094 | 78 | +52 | 1.67x |
| fix_ui_creative | fix_ui_creative.rs | 119 | 6330 | 67 | +52 | 1.78x |
| rename_blocks_items | rename_blocks_items.rs | 198 | 6874 | 147 | +51 | 1.35x |
| fix_ui_sub_hand | fix_ui_sub_hand.rs | 92 | 6397 | 44 | +48 | 2.09x |
| overlay_icons | overlay_icons.rs | 90 | 9275 | 49 | +41 | 1.84x |
| fix_horse_ui | fix_horse_ui.rs | 65 | 6770 | 104 | -39 | 0.63x |
| fix_ui_survival | fix_ui_survival.rs | 112 | 6183 | 147 | -35 | 0.76x |
| generate_fish_bucket | generate_fish_bucket.rs | 60 | 7220 | 94 | -34 | 0.64x |
| reverse_fix_clock_compass | reverse_fix_clock_compass.rs | 36 | 8715 | 9 | +27 | 4.00x |
| generate_smithing_ui | generate_smithing_ui.rs | 59 | 8003 | 85 | -26 | 0.69x |
| fix_slider | fix_slider.rs | 81 | 8172 | 59 | +22 | 1.37x |
| generate_potion_lingering | generate_potion_lingering.rs | 74 | 6498 | 96 | -22 | 0.77x |
| generate_copper_block | generate_copper.rs | 82 | 7637 | 103 | -21 | 0.80x |
| process_chest_folder | process_chest_folder.rs | 112 | 7440 | 91 | +21 | 1.23x |
| delete_blockstates_models | delete_blockstates_models.rs | 19 | 5893 | 4 | +15 | 4.75x |
| generate_crossbow | generate_crossbow.rs | 112 | 7314 | 126 | -14 | 0.89x |
| generate_snow_bucket | generate_snow_bucket.rs | 53 | 7946 | 39 | +14 | 1.36x |
| reverse_rename_blocks_items | reverse_rename_blocks_items.rs | 165 | 9054 | 151 | +14 | 1.09x |
| cut_gui | cut_gui.rs | 16 | 8510 | 29 | -13 | 0.55x |
| generate_netherite_ingot | generate_netherite.rs | 26 | 7569 | 39 | -13 | 0.67x |
| generate_netherite_block | generate_netherite.rs | 26 | 7531 | 38 | -12 | 0.68x |
| reverse_fix_particles | reverse_fix_particles.rs | 110 | 8933 | 121 | -11 | 0.91x |
| fix_sign_entities | fix_sign_entities.rs | 59 | 7085 | 69 | -10 | 0.86x |
| generate_netherite_tools | generate_netherite.rs | 57 | 7860 | 66 | -9 | 0.86x |
| rename_mcpatcher_to_optifine | rename_mcpatcher_to_optifine.rs | 27 | 7985 | 18 | +9 | 1.50x |
| fix2_horse_ui | fix2_horse_ui.rs | 34 | 8549 | 41 | -7 | 0.83x |
| fix_particles | fix_particles.rs | 13 | 7200 | 20 | -7 | 0.65x |

## ✅ rs 有但 pack.py 找不到(可能 py 端用不同名,或 pack.py 不存)

| converter | rs 文件 |
|---|---|
| fix_ui_survival_compat | fix_ui_survival.rs |
| register_task | cut_gui.rs |
| register_task_with_deps | cut_gui.rs |
| reverse_cut_gui | reverse_cut_gui.rs |
| reverse_fix_horse_ui | reverse_fix_horse_ui.rs |
| reverse_fix_machinery_ui | reverse_fix_machinery_ui.rs |
| reverse_fix_sign | reverse_fix_sign.rs |
| reverse_fix_sign_entities | reverse_fix_sign_entities.rs |
| reverse_fix_slider | reverse_fix_slider.rs |
| reverse_fix_smithing2_villager2_ui | reverse_fix_smithing2_villager2_ui.rs |
| reverse_fix_tabs | reverse_fix_tabs.rs |
| reverse_fix_ui_sub_hand | reverse_fix_ui_sub_hand.rs |
| reverse_fix2_horse_ui | reverse_fix2_horse_ui.rs |
| reverse_generate_boat | reverse_generate_boat.rs |
| reverse_generate_copper_armor_models | reverse_generate_copper.rs |
| reverse_generate_copper_block | reverse_generate_copper.rs |
| reverse_generate_copper_ingot | reverse_generate_copper.rs |
| reverse_generate_copper_tools | reverse_generate_copper.rs |
| reverse_generate_crossbow | reverse_generate_crossbow.rs |
| reverse_generate_fish_bucket | reverse_generate_fish_bucket.rs |
| reverse_generate_furnace | reverse_generate_furnace.rs |
| reverse_generate_netherite_armor_models | reverse_generate_netherite.rs |
| reverse_generate_netherite_block | reverse_generate_netherite.rs |
| reverse_generate_netherite_ingot | reverse_generate_netherite.rs |
| reverse_generate_netherite_tools | reverse_generate_netherite.rs |
| reverse_generate_pale_planks | reverse_generate_planks.rs |
| reverse_generate_potion_lingering | reverse_generate_potion_lingering.rs |
| reverse_generate_redwood_cherry_bamboo_planks | reverse_generate_planks.rs |
| reverse_generate_shulker_box_ui | reverse_generate_shulker_box_ui.rs |
| reverse_generate_smithing_ui | reverse_generate_smithing_ui.rs |
| reverse_generate_snow_bucket | reverse_generate_snow_bucket.rs |
| reverse_generate_tipped_arrow_images | reverse_generate_tipped_arrow_images.rs |
| reverse_overlay_icons | reverse_overlay_icons.rs |
| reverse_rename_mcpatcher_to_optifine | reverse_rename_mcpatcher_to_optifine.rs |

## ⚠️  pack.py 有但 rs 端没 converter(可能要新建 converter)

| py 函数 | py 行号 | 行数 |
|---|---:|---:|
| adjust_brightness_for_grayscale | 2866 | 13 |
| adjust_copper_color | 2662 | 29 |
| adjust_hue | 2879 | 8 |
| adjust_hue_brightness | 2608 | 54 |
| adjust_saturation | 2787 | 13 |
| average_color | 2135 | 11 |
| change_white_to_yellow | 2713 | 12 |
| clean_control_characters | 5536 | 8 |
| clean_non_json_content | 5544 | 68 |
| cleanup_residual_temp_dirs | 9665 | 51 |
| clear_frame | 1496 | 9 |
| color_fill_region | 5431 | 7 |
| combine_double_chest_images | 1774 | 196 |
| convert_to_zip | 126 | 183 |
| copy_and_adjust | 2764 | 23 |
| copy_and_paste_region | 5438 | 4 |
| create_mcmeta_file | 1628 | 17 |
| delete_folder | 5883 | 10 |
| detect_file_format | 84 | 42 |
| display_multiple_results | 5492 | 44 |
| display_result | 5457 | 35 |
| extract_zip | 5612 | 271 |
| fix_description | 2853 | 13 |
| generate_double_chest_images | 1663 | 111 |
| get_pack_format | 474 | 112 |
| hsv_to_rgba | 2586 | 22 |
| log | 72 | 12 |
| main_menu | 9633 | 32 |
| merge_images | 1599 | 29 |
| mirror_image_horizontally | 3163 | 4 |
| move_region | 5426 | 5 |
| new_pack_format_generate | 9324 | 93 |
| on_file_drop | 5442 | 9 |
| open_file_location | 326 | 22 |
| overlay_images | 3167 | 5 |
| process_block_image | 2800 | 53 |
| process_cartography_table_image | 2403 | 53 |
| process_grindstone_image | 2308 | 95 |
| process_icons_in_dir | 3209 | 257 |
| process_image | 2691 | 22 |
| process_loom_image | 2508 | 54 |
| process_redstone_dust_cross_image | 1970 | 20 |
| process_redstone_dust_line_image | 1990 | 15 |
| process_resource_packs_in_dir | 3825 | 137 |
| process_server_selection_in_dir | 4118 | 130 |
| process_stonecutter_image | 2456 | 52 |
| process_tabs_in_dir | 3650 | 175 |
| process_tipped_arrow | 3172 | 37 |
| process_title_in_dir | 3962 | 156 |
| process_villager_image | 2146 | 162 |
| process_widgets_in_dir | 3466 | 184 |
| process_zip | 9417 | 216 |
| process1_anvil_image | 4248 | 78 |
| process1_beacon_image | 4326 | 90 |
| process1_blast_furnace_image | 4416 | 71 |
| process1_brewing_stand_image | 4487 | 69 |
| process1_cartography_table_image | 4556 | 77 |
| process1_enchanting_table_image | 4633 | 93 |
| process1_furnace_image | 4726 | 68 |
| process1_grindstone_image | 4865 | 77 |
| process1_horse_image | 4942 | 73 |
| process1_inventory_image | 5015 | 66 |
| process1_loom_image | 5158 | 89 |
| process1_slider_image | 2017 | 102 |
| process1_smithing_image | 5081 | 77 |
| process1_smoker_image | 4794 | 71 |
| process1_stonecutter_image | 5247 | 89 |
| process1_villager2_image | 5336 | 90 |
| rename_and_process_blocks | 2887 | 276 |
| rename_items | 2725 | 39 |
| rgba_to_hsv | 2562 | 24 |
| scaled_coords | 2005 | 6 |
| scaled_point | 2011 | 6 |
| select_export_folder | 1505 | 9 |
| select_export_path | 819 | 14 |
| select_files | 5451 | 6 |
| set_default_theme | 309 | 4 |
| show_change_item_size | 1119 | 377 |
| show_conversion_options | 586 | 64 |
| show_main_menu | 313 | 13 |
| show_overlay_options | 854 | 54 |
| split_image | 1564 | 35 |
| split_particles_image | 1514 | 50 |
| start_bedrock_conversion | 650 | 138 |
| start_conversion | 788 | 31 |
| start_processing_conversion | 348 | 126 |
| start_processing_overlay | 908 | 211 |
| swap_and_mirror | 1652 | 11 |
| swap_rectangles | 1645 | 7 |
| update_overlay_options_ui | 833 | 21 |
