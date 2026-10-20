cmake_minimum_required(VERSION 3.14)
project(HeavenClient)

set(CMAKE_CXX_STANDARD 17)

option(HEAVENCLIENT_BUILD_TESTS "Build lightweight client logic tests" OFF)
if(HEAVENCLIENT_BUILD_TESTS)
    enable_testing()
    add_executable(quest_marker_tests tests/quest_marker.cpp)
    add_test(NAME quest_markers COMMAND quest_marker_tests)
    add_executable(quest_completion_tests tests/quest_completion.cpp)
    add_test(NAME quest_completion COMMAND quest_completion_tests)
    add_executable(fade_notice_tests tests/fade_notice.cpp)
    add_test(NAME fade_notice COMMAND fade_notice_tests)
    add_executable(input_focus_tests tests/input_focus.cpp)
    add_test(NAME input_focus COMMAND input_focus_tests)
    add_executable(movement_physics_tests tests/movement_physics.cpp
        Gameplay/Physics/Physics.cpp Gameplay/Physics/Foothold.cpp Gameplay/Physics/FootholdTree.cpp
        Gameplay/MapleMap/MapInfo.cpp)
    target_include_directories(movement_physics_tests PRIVATE ${PROJECT_SOURCE_DIR}/libs/NoLifeNx)
    target_link_libraries(movement_physics_tests PRIVATE
        ${PROJECT_SOURCE_DIR}/libs/NoLifeNx/nlnx/build/libNoLifeNx.a
        ${PROJECT_SOURCE_DIR}/libs/lz4/lib/liblz4.a)
    add_test(NAME movement_physics COMMAND movement_physics_tests)
    add_executable(map_objects_tests tests/map_objects.cpp
        Gameplay/MapleMap/MapObjects.cpp Gameplay/MapleMap/MapObject.cpp
        Gameplay/Physics/Physics.cpp Gameplay/Physics/Foothold.cpp Gameplay/Physics/FootholdTree.cpp)
    target_include_directories(map_objects_tests PRIVATE ${PROJECT_SOURCE_DIR}/libs/NoLifeNx)
    target_link_libraries(map_objects_tests PRIVATE
        ${PROJECT_SOURCE_DIR}/libs/NoLifeNx/nlnx/build/libNoLifeNx.a
        ${PROJECT_SOURCE_DIR}/libs/lz4/lib/liblz4.a)
    add_test(NAME map_objects COMMAND map_objects_tests)
    add_executable(frame_assembler_tests tests/frame_assembler.cpp Net/Cryptography.cpp)
    add_test(NAME frame_assembler COMMAND frame_assembler_tests)
    add_executable(quest_dialogue_tests tests/quest_dialogue.cpp)
    add_test(NAME quest_dialogue COMMAND quest_dialogue_tests)
    add_executable(npc_text_tests tests/npc_text.cpp)
    add_test(NAME npc_text COMMAND npc_text_tests)
    add_executable(npc_input_tests tests/npc_input.cpp)
    add_test(NAME npc_input COMMAND npc_input_tests)
    add_executable(slide_menu_tests tests/slide_menu.cpp)
    add_test(NAME slide_menu COMMAND slide_menu_tests)
    add_executable(quiz_tests tests/quiz.cpp)
    add_test(NAME quiz COMMAND quiz_tests)
    add_executable(world_map_lookup_tests tests/world_map_lookup.cpp)
    add_test(NAME world_map_lookup COMMAND world_map_lookup_tests)
    add_executable(swindle_words_tests tests/swindle_words.cpp)
    add_test(NAME swindle_words COMMAND swindle_words_tests)
    add_executable(text_layout_tests tests/text_layout.cpp Graphics/TextLayout.cpp)
    target_include_directories(text_layout_tests PRIVATE
        ${PROJECT_SOURCE_DIR}/libs/NoLifeNx ${PROJECT_SOURCE_DIR}/libs/glad/build/include)
    add_test(NAME text_layout COMMAND text_layout_tests)
    add_executable(chat_wrap_tests tests/chat_wrap.cpp)
    add_test(NAME chat_wrap COMMAND chat_wrap_tests)
    add_executable(megaphones_tests tests/megaphones.cpp)
    add_test(NAME megaphones COMMAND megaphones_tests)
    add_executable(chalkboard_tests tests/chalkboard.cpp)
    add_test(NAME chalkboard COMMAND chalkboard_tests)
    add_executable(teleport_rock_tests tests/teleport_rock.cpp Character/TeleportRock.cpp)
    add_test(NAME teleport_rock COMMAND teleport_rock_tests)
    add_executable(gacha_exp_tests tests/gacha_exp.cpp)
    add_test(NAME gacha_exp COMMAND gacha_exp_tests)
    add_executable(curse_words_tests tests/curse_words.cpp)
    add_test(NAME curse_words COMMAND curse_words_tests)
    add_executable(character_name_tests tests/character_name.cpp)
    add_test(NAME character_name COMMAND character_name_tests)
    add_executable(quest_result_tests tests/quest_result.cpp)
    add_test(NAME quest_result COMMAND quest_result_tests)
    add_executable(map_tooltip_tests tests/map_tooltip.cpp)
    add_test(NAME map_tooltip COMMAND map_tooltip_tests)
    add_executable(cash_catalog_tests tests/cash_catalog.cpp)
    add_test(NAME cash_catalog COMMAND cash_catalog_tests)
    add_executable(answer_focus_tests tests/answer_focus.cpp)
    add_test(NAME answer_focus COMMAND answer_focus_tests)
    add_executable(slider_steps_tests tests/slider_steps.cpp)
    add_test(NAME slider_steps COMMAND slider_steps_tests)
    add_executable(camera_follow_tests tests/camera_follow.cpp Gameplay/Camera.cpp)
    target_include_directories(camera_follow_tests PRIVATE ${PROJECT_SOURCE_DIR}/libs/NoLifeNx)
    add_test(NAME camera_follow COMMAND camera_follow_tests)
    add_executable(menu_cover_tests tests/menu_cover.cpp)
    target_include_directories(menu_cover_tests PRIVATE ${PROJECT_SOURCE_DIR}/libs/NoLifeNx)
    add_test(NAME menu_cover COMMAND menu_cover_tests)
    add_executable(party_tests tests/party.cpp Character/Party.cpp)
    target_include_directories(party_tests PRIVATE ${PROJECT_SOURCE_DIR}/libs/NoLifeNx)
    add_test(NAME party COMMAND party_tests)
    add_executable(door_rules_tests tests/door_rules.cpp Character/Party.cpp)
    target_include_directories(door_rules_tests PRIVATE ${PROJECT_SOURCE_DIR}/libs/NoLifeNx)
    add_test(NAME door_rules COMMAND door_rules_tests)
    add_executable(buddy_list_tests tests/buddy_list.cpp Character/BuddyList.cpp)
    add_test(NAME buddy_list COMMAND buddy_list_tests)
    add_executable(guild_tests tests/guild.cpp Character/Guild.cpp Net/InPacket.cpp Net/Handlers/Helpers/GuildParser.cpp)
    target_include_directories(guild_tests PRIVATE ${PROJECT_SOURCE_DIR}/libs/NoLifeNx)
    add_test(NAME guild COMMAND guild_tests)
    add_executable(npc_reach_tests tests/npc_reach.cpp)
    target_include_directories(npc_reach_tests PRIVATE ${PROJECT_SOURCE_DIR}/libs/NoLifeNx)
    add_test(NAME npc_reach COMMAND npc_reach_tests)
    add_executable(npc_idle_tests tests/npc_idle.cpp)
    add_test(NAME npc_idle COMMAND npc_idle_tests)
    add_executable(monster_book_tests tests/monster_book.cpp Character/MonsterBook.cpp)
    add_test(NAME monster_book COMMAND monster_book_tests)
    add_executable(enter_number_tests tests/enter_number.cpp)
    add_test(NAME enter_number COMMAND enter_number_tests)
    add_executable(keyboard_tests tests/keyboard.cpp IO/Keyboard.cpp)
    add_test(NAME keyboard COMMAND keyboard_tests)
    add_executable(mob_damage_tests tests/mob_damage.cpp)
    add_test(NAME mob_damage COMMAND mob_damage_tests)
    add_executable(mob_attack_tests tests/mob_attack.cpp)
    target_include_directories(mob_attack_tests PRIVATE ${PROJECT_SOURCE_DIR}/libs/NoLifeNx)
    add_test(NAME mob_attack COMMAND mob_attack_tests)
    add_executable(magic_damage_tests tests/magic_damage.cpp)
    add_test(NAME magic_damage COMMAND magic_damage_tests)
    add_executable(physical_damage_tests tests/physical_damage.cpp)
    add_test(NAME physical_damage COMMAND physical_damage_tests)
    add_executable(secondary_stat_tests tests/secondary_stat.cpp)
    add_test(NAME secondary_stat COMMAND secondary_stat_tests)
    add_executable(buff_row_tests tests/buff_row.cpp)
    add_test(NAME buff_row COMMAND buff_row_tests)
    add_executable(item_drop_tests tests/item_drop.cpp)
    add_test(NAME item_drop COMMAND item_drop_tests)
    add_executable(field_limit_tests tests/field_limit.cpp)
    add_test(NAME field_limit COMMAND field_limit_tests)
    add_executable(scroll_rules_tests tests/scroll_rules.cpp)
    add_test(NAME scroll_rules COMMAND scroll_rules_tests)
    add_executable(pickup_tests tests/pickup.cpp)
    target_include_directories(pickup_tests PRIVATE ${PROJECT_SOURCE_DIR}/libs/NoLifeNx)
    add_test(NAME pickup COMMAND pickup_tests)
    add_executable(damage_layout_tests tests/damage_layout.cpp)
    add_test(NAME damage_layout COMMAND damage_layout_tests)
    add_executable(hit_timing_tests tests/hit_timing.cpp)
    target_include_directories(hit_timing_tests PRIVATE ${PROJECT_SOURCE_DIR}/libs/NoLifeNx)
    add_test(NAME hit_timing COMMAND hit_timing_tests)
    add_executable(attack_stances_tests tests/attack_stances.cpp)
    target_include_directories(attack_stances_tests PRIVATE ${PROJECT_SOURCE_DIR}/Character)
    add_test(NAME attack_stances COMMAND attack_stances_tests)
    add_executable(point_blank_tests tests/point_blank.cpp)
    target_include_directories(point_blank_tests PRIVATE ${PROJECT_SOURCE_DIR}/libs/NoLifeNx)
    add_test(NAME point_blank COMMAND point_blank_tests)
    add_executable(shot_path_tests tests/shot_path.cpp)
    target_include_directories(shot_path_tests PRIVATE ${PROJECT_SOURCE_DIR}/libs/NoLifeNx)
    add_test(NAME shot_path COMMAND shot_path_tests)
    add_executable(chain_lightning_tests tests/chain_lightning.cpp)
    target_include_directories(chain_lightning_tests PRIVATE ${PROJECT_SOURCE_DIR}/libs/NoLifeNx)
    add_test(NAME chain_lightning COMMAND chain_lightning_tests)
    add_executable(remote_spell_tests tests/remote_spell.cpp)
    target_include_directories(remote_spell_tests PRIVATE ${PROJECT_SOURCE_DIR}/libs/NoLifeNx)
    add_test(NAME remote_spell COMMAND remote_spell_tests)
    add_executable(hit_point_tests tests/hit_point.cpp)
    target_include_directories(hit_point_tests PRIVATE ${PROJECT_SOURCE_DIR}/libs/NoLifeNx)
    add_test(NAME hit_point COMMAND hit_point_tests)
    add_executable(remote_shot_tests tests/remote_shot.cpp)
    target_include_directories(remote_shot_tests PRIVATE ${PROJECT_SOURCE_DIR}/libs/NoLifeNx)
    add_test(NAME remote_shot COMMAND remote_shot_tests)
    add_executable(shot_rain_tests tests/shot_rain.cpp)
    target_include_directories(shot_rain_tests PRIVATE ${PROJECT_SOURCE_DIR}/libs/NoLifeNx)
    add_test(NAME shot_rain COMMAND shot_rain_tests)
    add_executable(spell_area_tests tests/spell_area.cpp)
    target_include_directories(spell_area_tests PRIVATE ${PROJECT_SOURCE_DIR}/libs/NoLifeNx)
    add_test(NAME spell_area COMMAND spell_area_tests)
    add_executable(heal_tests tests/heal.cpp)
    target_include_directories(heal_tests PRIVATE ${PROJECT_SOURCE_DIR}/libs/NoLifeNx)
    add_test(NAME heal COMMAND heal_tests)
    add_executable(combo_tests tests/combo.cpp)
    add_test(NAME combo COMMAND combo_tests)
    add_executable(combo_orbs_tests tests/combo_orbs.cpp)
    add_test(NAME combo_orbs COMMAND combo_orbs_tests)
    add_executable(final_attack_tests tests/final_attack.cpp)
    add_test(NAME final_attack COMMAND final_attack_tests)
    add_executable(teleport_tests tests/teleport.cpp)
    target_include_directories(teleport_tests PRIVATE ${PROJECT_SOURCE_DIR}/libs/NoLifeNx)
    add_test(NAME teleport COMMAND teleport_tests)
    add_executable(flash_jump_tests tests/flash_jump.cpp)
    add_test(NAME flash_jump COMMAND flash_jump_tests)
    add_executable(prepare_tests tests/prepare.cpp)
    target_include_directories(prepare_tests PRIVATE ${PROJECT_SOURCE_DIR}/libs/NoLifeNx)
    add_test(NAME prepare COMMAND prepare_tests)
    add_executable(skill_use_tests tests/skill_use.cpp)
    add_test(NAME skill_use COMMAND skill_use_tests)
    add_executable(magnet_tests tests/magnet.cpp)
    add_test(NAME magnet COMMAND magnet_tests)
    add_executable(meso_explosion_tests tests/meso_explosion.cpp)
    target_include_directories(meso_explosion_tests PRIVATE ${PROJECT_SOURCE_DIR}/libs/NoLifeNx)
    add_test(NAME meso_explosion COMMAND meso_explosion_tests)
    add_executable(timed_queue_tests tests/timed_queue.cpp)
    add_test(NAME timed_queue COMMAND timed_queue_tests)
    add_executable(shop_list_tests tests/shop_list.cpp Net/InPacket.cpp)
    target_include_directories(shop_list_tests PRIVATE ${PROJECT_SOURCE_DIR}/libs/NoLifeNx)
    add_test(NAME shop_list COMMAND shop_list_tests)
    add_executable(storage_contents_tests tests/storage_contents.cpp Net/InPacket.cpp)
    target_include_directories(storage_contents_tests PRIVATE ${PROJECT_SOURCE_DIR}/libs/NoLifeNx)
    add_test(NAME storage_contents COMMAND storage_contents_tests)
    add_executable(mini_room_tests tests/mini_room.cpp Net/InPacket.cpp)
    target_include_directories(mini_room_tests PRIVATE ${PROJECT_SOURCE_DIR}/libs/NoLifeNx)
    add_test(NAME mini_room COMMAND mini_room_tests)
    add_executable(personal_shop_tests tests/personal_shop.cpp Net/InPacket.cpp)
    target_include_directories(personal_shop_tests PRIVATE ${PROJECT_SOURCE_DIR}/libs/NoLifeNx)
    add_test(NAME personal_shop COMMAND personal_shop_tests)
    add_executable(movement_path_tests tests/movement_path.cpp Net/InPacket.cpp Net/Handlers/Helpers/MovementParser.cpp)
    target_include_directories(movement_path_tests PRIVATE ${PROJECT_SOURCE_DIR}/libs/NoLifeNx)
    add_test(NAME movement_path COMMAND movement_path_tests)
    add_executable(pets_tests tests/pets.cpp Net/InPacket.cpp)
    target_include_directories(pets_tests PRIVATE ${PROJECT_SOURCE_DIR}/libs/NoLifeNx)
    add_test(NAME pets COMMAND pets_tests)
    add_executable(buff_parser_tests tests/buff_parser.cpp Net/InPacket.cpp Net/Handlers/Helpers/BuffParser.cpp Character/Buff.cpp)
    target_include_directories(buff_parser_tests PRIVATE ${PROJECT_SOURCE_DIR}/libs/NoLifeNx)
    add_test(NAME buff_parser COMMAND buff_parser_tests)
    add_executable(diseases_tests tests/diseases.cpp)
    target_include_directories(diseases_tests PRIVATE ${PROJECT_SOURCE_DIR}/libs/NoLifeNx)
    add_test(NAME diseases COMMAND diseases_tests)
    add_executable(mob_stat_parser_tests tests/mob_stat_parser.cpp Net/InPacket.cpp Net/Handlers/Helpers/MobStatParser.cpp Net/Handlers/Helpers/BuffParser.cpp Character/Buff.cpp)
    target_include_directories(mob_stat_parser_tests PRIVATE ${PROJECT_SOURCE_DIR}/libs/NoLifeNx)
    add_test(NAME mob_stat_parser COMMAND mob_stat_parser_tests)
    add_executable(mob_hurts_tests tests/mob_hurts.cpp)
    target_include_directories(mob_hurts_tests PRIVATE ${PROJECT_SOURCE_DIR}/libs/NoLifeNx)
    add_test(NAME mob_hurts COMMAND mob_hurts_tests)
    add_executable(mist_layout_tests tests/mist_layout.cpp)
    add_test(NAME mist_layout COMMAND mist_layout_tests)
    add_executable(obstacle_rules_tests tests/obstacle_rules.cpp)
    add_test(NAME obstacle_rules COMMAND obstacle_rules_tests)
    add_executable(tremble_rules_tests tests/tremble_rules.cpp)
    add_test(NAME tremble_rules COMMAND tremble_rules_tests)
    add_executable(field_effect_rules_tests tests/field_effect_rules.cpp)
    add_test(NAME field_effect_rules COMMAND field_effect_rules_tests)
    add_executable(weather_rules_tests tests/weather_rules.cpp)
    add_test(NAME weather_rules COMMAND weather_rules_tests)
    add_executable(item_effect_rules_tests tests/item_effect_rules.cpp)
    add_test(NAME item_effect_rules COMMAND item_effect_rules_tests)
    add_executable(ap_reset_rules_tests tests/ap_reset_rules.cpp)
    add_test(NAME ap_reset_rules COMMAND ap_reset_rules_tests)
    add_executable(sp_reset_rules_tests tests/sp_reset_rules.cpp)
    add_test(NAME sp_reset_rules COMMAND sp_reset_rules_tests)
    add_executable(item_expiry_tests tests/item_expiry.cpp)
    add_test(NAME item_expiry COMMAND item_expiry_tests)
    add_executable(skill_macros_tests tests/skill_macros.cpp)
    add_test(NAME skill_macros COMMAND skill_macros_tests)
    add_executable(skill_macro_rules_tests tests/skill_macro_rules.cpp)
    add_test(NAME skill_macro_rules COMMAND skill_macro_rules_tests)
    add_executable(skill_macro_layout_tests tests/skill_macro_layout.cpp)
    add_test(NAME skill_macro_layout COMMAND skill_macro_layout_tests)
    add_executable(mount_rules_tests tests/mount_rules.cpp)
    add_test(NAME mount_rules COMMAND mount_rules_tests)
    add_executable(catch_items_tests tests/catch_items.cpp)
    add_test(NAME catch_items COMMAND catch_items_tests)
    add_executable(obj_flow_tests tests/obj_flow.cpp)
    add_test(NAME obj_flow COMMAND obj_flow_tests)
    add_executable(item_properties_tests tests/item_properties.cpp)
    add_test(NAME item_properties COMMAND item_properties_tests)
    add_executable(obj_state_rules_tests tests/obj_state_rules.cpp)
    target_include_directories(obj_state_rules_tests PRIVATE
        ${PROJECT_SOURCE_DIR}/libs/NoLifeNx ${PROJECT_SOURCE_DIR}/libs/glad/build/include)
    add_test(NAME obj_state_rules COMMAND obj_state_rules_tests)
    add_executable(map_clouds_tests tests/map_clouds.cpp)
    add_test(NAME map_clouds COMMAND map_clouds_tests)
    add_executable(map_tooltip_rules_tests tests/map_tooltip_rules.cpp)
    add_test(NAME map_tooltip_rules COMMAND map_tooltip_rules_tests)
    add_executable(screenshot_rules_tests tests/screenshot_rules.cpp)
    add_test(NAME screenshot_rules COMMAND screenshot_rules_tests)
    add_executable(medal_tag_layout_tests tests/medal_tag_layout.cpp)
    add_test(NAME medal_tag_layout COMMAND medal_tag_layout_tests)
    add_executable(mob_gage_layout_tests tests/mob_gage_layout.cpp)
    add_test(NAME mob_gage_layout COMMAND mob_gage_layout_tests)
    add_executable(ships_tests tests/ships.cpp)
    add_test(NAME ships COMMAND ships_tests)
    add_executable(field_clock_tests tests/field_clock.cpp)
    add_test(NAME field_clock COMMAND field_clock_tests)
    add_executable(reactor_rules_tests tests/reactor_rules.cpp)
    add_test(NAME reactor_rules COMMAND reactor_rules_tests)
    add_executable(status_messages_tests tests/status_messages.cpp Net/InPacket.cpp)
    target_include_directories(status_messages_tests PRIVATE ${PROJECT_SOURCE_DIR}/libs/NoLifeNx)
    add_test(NAME status_messages COMMAND status_messages_tests)
    add_executable(skill_recommendation_tests tests/skill_recommendation.cpp)
    add_test(NAME skill_recommendation COMMAND skill_recommendation_tests)
    add_executable(emote_cooldown_tests tests/emote_cooldown.cpp)
    add_test(NAME emote_cooldown COMMAND emote_cooldown_tests)
    add_executable(fame_cooldown_tests tests/fame_cooldown.cpp)
    add_test(NAME fame_cooldown COMMAND fame_cooldown_tests)
    add_executable(summon_rules_tests tests/summon_rules.cpp)
    add_test(NAME summon_rules COMMAND summon_rules_tests)
    add_executable(chat_flood_guard_tests tests/chat_flood_guard.cpp)
    add_test(NAME chat_flood_guard COMMAND chat_flood_guard_tests)
    add_executable(char_click_tests tests/char_click.cpp)
    target_include_directories(char_click_tests PRIVATE ${PROJECT_SOURCE_DIR}/libs/NoLifeNx)
    add_test(NAME char_click COMMAND char_click_tests)
    add_executable(context_menu_tests tests/context_menu.cpp)
    target_include_directories(context_menu_tests PRIVATE ${PROJECT_SOURCE_DIR}/libs/NoLifeNx)
    add_test(NAME context_menu COMMAND context_menu_tests)
    add_executable(excl_request_tests tests/excl_request.cpp)
    add_test(NAME excl_request COMMAND excl_request_tests)
    add_executable(portal_touch_tests tests/portal_touch.cpp Gameplay/MapleMap/Portal.cpp)
    target_include_directories(portal_touch_tests PRIVATE
        ${PROJECT_SOURCE_DIR}/libs/NoLifeNx ${PROJECT_SOURCE_DIR}/libs/glad/build/include)
    add_test(NAME portal_touch COMMAND portal_touch_tests)
    # Loads every quest in game-data/supported_quests.txt from Quest.nx.
    # Skipped when Quest.nx is absent.
    add_executable(quest_catalog_tests tests/quest_catalog.cpp Character/QuestCatalog.cpp)
    target_include_directories(quest_catalog_tests PRIVATE ${PROJECT_SOURCE_DIR}/libs/NoLifeNx ${PROJECT_SOURCE_DIR}/includes)
    target_link_libraries(quest_catalog_tests PRIVATE
        ${PROJECT_SOURCE_DIR}/libs/NoLifeNx/nlnx/build/libNoLifeNx.a
        ${PROJECT_SOURCE_DIR}/libs/lz4/lib/liblz4.a)
    target_compile_definitions(quest_catalog_tests PRIVATE
        SUPPORTED_QUESTS="${PROJECT_SOURCE_DIR}/../game-data/supported_quests.txt"
        DEFAULT_QUEST_NX="${PROJECT_SOURCE_DIR}/../assets/game-data/Quest.nx")
    add_test(NAME quest_catalog COMMAND quest_catalog_tests)
    add_executable(skill_weapon_tests tests/skill_weapon.cpp)
    add_test(NAME skill_weapon COMMAND skill_weapon_tests)
endif()

include_directories(.)
include_directories(Audio)
include_directories(Character)
include_directories(Character/Inventory)
include_directories(Character/Look)
include_directories(Data)
include_directories(Gameplay)
include_directories(Gameplay/Combat)
include_directories(Gameplay/MapleMap)
include_directories(Gameplay/Physics)
include_directories(Graphics)
include_directories(IO)
include_directories(IO/Components)
include_directories(IO/UITypes)
include_directories(Net)
include_directories(Net/Handlers)
include_directories(Net/Handlers/Helpers)
include_directories(Net/Packets)
include_directories(Template)
include_directories(Util)

add_executable(HeavenClient
    Audio/Audio.cpp
    Audio/Audio.h
    Character/Inventory/Equip.cpp
    Character/Inventory/Equip.h
    Character/Inventory/EquipQuality.cpp
    Character/Inventory/EquipQuality.h
    Character/Inventory/Inventory.cpp
    Character/Inventory/Inventory.h
    Character/Inventory/InventoryType.cpp
    Character/Inventory/InventoryType.h
    Character/Inventory/Item.cpp
    Character/Inventory/Item.h
    Character/Inventory/ItemExpiry.h
    Character/Inventory/ItemProperties.h
    Character/Inventory/Pet.cpp
    Character/Inventory/Pet.h
    Character/Inventory/Weapon.cpp
    Character/Inventory/Weapon.h
    Character/Look/Afterimage.cpp
    Character/Look/Afterimage.h
    Character/Look/Body.cpp
    Character/Look/Body.h
    Character/Look/BodyDrawInfo.cpp
    Character/Look/BodyDrawInfo.h
    Character/Look/CharEquips.cpp
    Character/Look/CharEquips.h
    Character/Look/CharLook.cpp
    Character/Look/CharLook.h
    Character/Look/Clothing.cpp
    Character/Look/Clothing.h
    Character/Look/EquipSlot.cpp
    Character/Look/EquipSlot.h
    Character/Look/Face.cpp
    Character/Look/Face.h
    Character/Look/Hair.cpp
    Character/Look/Hair.h
    Character/Look/PetLook.cpp
    Character/Look/PetLook.h
    Character/Look/Stance.cpp
    Character/Look/Stance.h
    Character/Look/TamingMob.cpp
    Character/Look/TamingMob.h
    Character/ActiveBuffs.cpp
    Character/ActiveBuffs.h
    Character/BuddyList.cpp
    Character/BuddyList.h
    Character/Buff.cpp
    Character/Buff.h
    Character/ChalkboardBalloon.cpp
    Character/ChalkboardBalloon.h
    Character/Char.cpp
    Character/Char.h
    Character/MedalTag.cpp
    Character/MedalTag.h
    Character/MedalTagLayout.h
    Character/CharEffect.cpp
    Character/CharEffect.h
    Character/ItemEffect.cpp
    Character/ItemEffect.h
    Character/ItemEffectRules.h
    Character/ApResetRules.h
    Character/SpResetRules.h
    Character/CharStats.cpp
    Character/CharStats.h
    Character/EquipStat.cpp
    Character/EquipStat.h
    Character/ExpTable.cpp
    Character/ExpTable.h
    Character/Guild.cpp
    Character/Guild.h
    Character/GuildMark.cpp
    Character/GuildMark.h
    Character/Job.cpp
    Character/Job.h
    Character/MapleStat.cpp
    Character/MapleStat.h
    Character/MonsterBook.cpp
    Character/MonsterBook.h
    Character/MountRules.h
    Character/Party.cpp
    Character/Party.h
    Character/Pets.h
    Character/MiniRoomBalloon.cpp
    Character/MiniRoomBalloon.h
    Character/MiniRoomBox.h
    Character/OtherChar.cpp
    Character/OtherChar.h
    Character/PassiveBuffs.cpp
    Character/PassiveBuffs.h
    Character/Player.cpp
    Character/Player.h
    Character/PlayerStates.cpp
    Character/PlayerStates.h
    Character/QuestCatalog.cpp
    Character/QuestCatalog.h
    Character/QuestCompletion.h
    Character/QuestDialogue.h
    Character/QuestInfo.h
    Character/QuestLog.cpp
    Character/QuestLog.h
    Character/QuestResult.h
    Character/SkillBook.cpp
    Character/SkillBook.h
    Character/SkillId.h
    Character/StatCaps.h
    Character/TeleportRock.cpp
    Character/TeleportRock.h
    Data/BulletData.cpp
    Data/BulletData.h
    Data/CurseData.cpp
    Data/CurseData.h
    Data/EquipData.cpp
    Data/EquipData.h
    Data/ItemData.cpp
    Data/ItemData.h
    Data/CashCatalog.h
    Data/JobData.cpp
    Data/JobData.h
    Data/PetData.cpp
    Data/PetData.h
    Data/SkillData.cpp
    Data/SkillData.h
    Data/SwindleData.cpp
    Data/SwindleData.h
    Data/WeaponData.cpp
    Data/WeaponData.h
    Gameplay/Combat/Attack.h
    Gameplay/Combat/Bullet.cpp
    Gameplay/Combat/Bullet.h
    Gameplay/Combat/Combat.cpp
    Gameplay/Combat/Combat.h
    Gameplay/Combat/DamageNumber.cpp
    Gameplay/Combat/DamageNumber.h
    Gameplay/Combat/RegularAttack.cpp
    Gameplay/Combat/RegularAttack.h
    Gameplay/Combat/Skill.cpp
    Gameplay/Combat/Skill.h
    Gameplay/Combat/SkillAction.cpp
    Gameplay/Combat/SkillAction.h
    Gameplay/Combat/SkillBullet.cpp
    Gameplay/Combat/SkillBullet.h
    Gameplay/Combat/SkillHitEffect.cpp
    Gameplay/Combat/SkillHitEffect.h
    Gameplay/Combat/SkillSound.cpp
    Gameplay/Combat/SkillSound.h
    Gameplay/Combat/SkillUseEffect.cpp
    Gameplay/Combat/SkillUseEffect.h
    Gameplay/Combat/SpecialMove.h
    Gameplay/MapleMap/Drop.cpp
    Gameplay/MapleMap/Drop.h
    Gameplay/MapleMap/ItemDrop.cpp
    Gameplay/MapleMap/ItemDrop.h
    Gameplay/MapleMap/Layer.h
    Gameplay/MapleMap/MapBackgrounds.cpp
    Gameplay/MapleMap/MapBackgrounds.h
    Gameplay/MapleMap/MapChars.cpp
    Gameplay/MapleMap/MapChars.h
    Gameplay/MapleMap/MapClock.cpp
    Gameplay/MapleMap/MapClock.h
    Gameplay/MapleMap/ClockRules.h
    Gameplay/MapleMap/MapDrops.cpp
    Gameplay/MapleMap/MapDrops.h
    Gameplay/MapleMap/MapInfo.cpp
    Gameplay/MapleMap/MapInfo.h
    Gameplay/MapleMap/CloudLayout.h
    Gameplay/MapleMap/ObjFlow.h
    Gameplay/MapleMap/MapClouds.cpp
    Gameplay/MapleMap/MapClouds.h
    Gameplay/MapleMap/MapWeather.cpp
    Gameplay/MapleMap/MapWeather.h
    Gameplay/WeatherRules.h
    Gameplay/MapleMap/MapMists.cpp
    Gameplay/MapleMap/MapDoors.cpp
    Gameplay/MapleMap/MapMists.h
    Gameplay/MapleMap/MapSummons.cpp
    Gameplay/MapleMap/MapSummons.h
    Gameplay/MapleMap/SummonRules.h
    Gameplay/MapleMap/MapMobs.cpp
    Gameplay/MapleMap/MapMobs.h
    Gameplay/MapleMap/MapNpcs.cpp
    Gameplay/MapleMap/MapNpcs.h
    Gameplay/MapleMap/MapObject.cpp
    Gameplay/MapleMap/MapObject.h
    Gameplay/MapleMap/MapObjects.cpp
    Gameplay/MapleMap/MapObjects.h
    Gameplay/MapleMap/MapPortals.cpp
    Gameplay/MapleMap/MapPortals.h
    Gameplay/MapleMap/MapReactors.cpp
    Gameplay/MapleMap/MapReactors.h
    Gameplay/MapleMap/MapShip.cpp
    Gameplay/MapleMap/MapShip.h
    Gameplay/MapleMap/ShipRules.h
    Gameplay/MapleMap/MapTilesObjs.cpp
    Gameplay/MapleMap/MapTilesObjs.h
    Gameplay/MapleMap/MapTooltipRules.h
    Gameplay/MapleMap/MapTooltips.cpp
    Gameplay/MapleMap/MapTooltips.h
    Gameplay/MapleMap/MesoDrop.cpp
    Gameplay/MapleMap/MesoDrop.h
    Gameplay/MapleMap/MistLayout.h
    Gameplay/MapleMap/ObstacleRules.h
    Gameplay/MapleMap/Mob.cpp
    Gameplay/MapleMap/Mob.h
    Gameplay/MapleMap/Npc.cpp
    Gameplay/MapleMap/Npc.h
    Gameplay/MapleMap/Obj.cpp
    Gameplay/MapleMap/Obj.h
    Gameplay/MapleMap/ObjStateRules.h
    Gameplay/MapleMap/Portal.cpp
    Gameplay/MapleMap/Portal.h
    Gameplay/MapleMap/Reactor.cpp
    Gameplay/MapleMap/Reactor.h
    Gameplay/MapleMap/Tile.cpp
    Gameplay/MapleMap/Tile.h
    Gameplay/Physics/Foothold.cpp
    Gameplay/Physics/Foothold.h
    Gameplay/Physics/FootholdTree.cpp
    Gameplay/Physics/FootholdTree.h
    Gameplay/Physics/Physics.cpp
    Gameplay/Physics/Physics.h
    Gameplay/Physics/PhysicsObject.h
    Gameplay/Camera.cpp
    Gameplay/Camera.h
    Gameplay/Movement.h
    Gameplay/Playable.h
    Gameplay/Spawn.cpp
    Gameplay/Spawn.h
    Gameplay/QuestConversation.cpp
    Gameplay/QuestConversation.h
    Gameplay/Stage.cpp
    Gameplay/Stage.h
    Gameplay/TrembleRules.h
    Gameplay/FieldEffectRules.h
    Graphics/Animation.cpp
    Graphics/Animation.h
    Graphics/Color.cpp
    Graphics/Color.h
    Graphics/DrawArgument.h
    Graphics/EffectLayer.cpp
    Graphics/EffectLayer.h
    Graphics/Geometry.cpp
    Graphics/Geometry.h
    Graphics/GraphicsGL.cpp
    Graphics/GraphicsGL.h
    Graphics/SpecialText.h
    Graphics/Sprite.cpp
    Graphics/Sprite.h
    Graphics/Text.cpp
    Graphics/Text.h
    Graphics/TextLayout.cpp
    Graphics/TextLayout.h
    Graphics/Utf8.h
    Graphics/Texture.cpp
    Graphics/Texture.h
    IO/Components/AreaButton.cpp
    IO/Components/AreaButton.h
    IO/Components/BuffRow.h
    IO/Components/Button.cpp
    IO/Components/Button.h
    IO/Components/Charset.cpp
    IO/Components/Charset.h
    IO/Components/ChatBalloon.cpp
    IO/Components/ChatBalloon.h
    IO/Components/ClassicSlider.cpp
    IO/Components/ClassicSlider.h
    IO/Components/ClassicTabs.cpp
    IO/Components/ClassicTabs.h
    IO/Components/SliderSteps.h
    IO/Components/EquipTooltip.cpp
    IO/Components/EquipTooltip.h
    IO/Components/ContextMenuLayout.h
    IO/Components/FadeNotice.h
    IO/Components/Gauge.cpp
    IO/Components/Gauge.h
    IO/Components/Icon.cpp
    IO/Components/Icon.h
    IO/Components/ItemTooltip.cpp
    IO/Components/ItemTooltip.h
    IO/Components/MapleButton.cpp
    IO/Components/MapleButton.h
    IO/Components/MapleComboBox.cpp
    IO/Components/MapleComboBox.h
    IO/Components/MapleFrame.cpp
    IO/Components/MapleFrame.h
    IO/Components/MapTooltip.cpp
    IO/Components/MapTooltip.h
    IO/Components/NameTag.cpp
    IO/Components/NameTag.h
    IO/Components/NpcText.cpp
    IO/Components/PlainTooltip.cpp
    IO/Components/PlainTooltip.h
    IO/Components/ScrollingNotice.cpp
    IO/Components/ScrollingNotice.h
    IO/Components/SkillTooltip.cpp
    IO/Components/SkillTooltip.h
    IO/Components/Slider.cpp
    IO/Components/Slider.h
    IO/Components/StatefulIcon.cpp
    IO/Components/StatefulIcon.h
    IO/Components/Textfield.cpp
    IO/Components/Textfield.h
    IO/Components/TextTooltip.cpp
    IO/Components/TextTooltip.h
    IO/Components/TooltipBox.cpp
    IO/Components/TooltipBox.h
    IO/Components/Tooltip.h
    IO/Components/TwoSpriteButton.cpp
    IO/Components/TwoSpriteButton.h
    IO/UITypes/UIBuffList.cpp
    IO/UITypes/UIBuffList.h
    IO/UITypes/UIQuestHelper.cpp
    IO/UITypes/UIQuestComplete.cpp
    IO/UITypes/UIQuestComplete.h
    IO/UITypes/UIPartyHP.cpp
    IO/UITypes/UIPartyHP.h
    IO/UITypes/UIQuestHelper.h
    IO/UITypes/UICashBuy.cpp
    IO/UITypes/UICashBuy.h
    IO/UITypes/UICashGift.cpp
    IO/UITypes/UICashGift.h
    IO/UITypes/UIGiftNotice.cpp
    IO/UITypes/UIGiftNotice.h
    IO/UITypes/UICashShop.cpp
    IO/UITypes/UICashShop.h
    IO/UITypes/UIChannel.cpp
    IO/UITypes/UIChannel.h
    IO/UITypes/UIGameOption.cpp
    IO/UITypes/UIGameOption.h
    IO/UITypes/UICharInfo.cpp
    IO/UITypes/UICharInfo.h
    IO/UITypes/UIContextMenu.cpp
    IO/UITypes/UIContextMenu.h
    IO/UITypes/UICharCreation.cpp
    IO/UITypes/UICharCreation.h
    IO/UITypes/UICharSelect.cpp
    IO/UITypes/UICharSelect.h
    IO/UITypes/UIChatBar.cpp
    IO/UITypes/UIChatBar.h
    IO/UITypes/UIEnterText.h
    IO/UITypes/UIEquipInventory.cpp
    IO/UITypes/UIEquipInventory.h
    IO/UITypes/UIGender.cpp
    IO/UITypes/UIGender.h
    IO/UITypes/UIItemInventory.cpp
    IO/UITypes/UIItemInventory.h
    IO/UITypes/UIKeyConfig.cpp
    IO/UITypes/UIKeyConfig.h
    IO/UITypes/UILogin.cpp
    IO/UITypes/UILogin.h
    IO/UITypes/UILoginNotice.cpp
    IO/UITypes/UILoginNotice.h
    IO/UITypes/UILoginWait.cpp
    IO/UITypes/UILoginWait.h
    IO/UITypes/UILogo.cpp
    IO/UITypes/UILogo.h
    IO/UITypes/UIMessenger.cpp
    IO/UITypes/UIMessenger.h
    IO/UITypes/UIMiniMap.cpp
    IO/UITypes/UIMiniMap.h
    IO/UITypes/MobGageLayout.h
    IO/UITypes/UIMobGage.cpp
    IO/UITypes/UIMobGage.h
    IO/UITypes/EnterNumber.h
    IO/UITypes/UINotice.cpp
    IO/UITypes/UINotice.h
    IO/UITypes/UIAvatarSelect.cpp
    IO/UITypes/UIAvatarSelect.h
    IO/UITypes/SlideMenu.h
    IO/UITypes/UINpcTalk.cpp
    IO/UITypes/UINpcTalk.h
    IO/UITypes/Quiz.h
    IO/UITypes/UIQuestLog.cpp
    IO/UITypes/UIQuestLog.h
    IO/UITypes/UIQuiz.cpp
    IO/UITypes/UIQuiz.h
    IO/UITypes/UIFieldClock.cpp
    IO/UITypes/UIFieldClock.h
    IO/UITypes/UIQuit.cpp
    IO/UITypes/UIQuit.h
    IO/UITypes/UIRaceSelect.cpp
    IO/UITypes/UIRaceSelect.h
    IO/UITypes/UIRevive.cpp
    IO/UITypes/UIRevive.h
    IO/UITypes/UIShop.cpp
    IO/UITypes/UIShop.h
    IO/UITypes/UISlideMenu.cpp
    IO/UITypes/UISlideMenu.h
    IO/UITypes/UIStorage.cpp
    IO/UITypes/UIStorage.h
    IO/UITypes/UITeleportRock.cpp
    IO/UITypes/UITeleportRock.h
    IO/UITypes/UIMegaphone.cpp
    IO/UITypes/UIMegaphone.h
    IO/UITypes/UIPetChooser.cpp
    IO/UITypes/UIPetChooser.h
    IO/UITypes/UIApReset.cpp
    IO/UITypes/UIApReset.h
    IO/UITypes/UISpReset.cpp
    IO/UITypes/UISpReset.h
    IO/UITypes/UIItemMegaphone.cpp
    IO/UITypes/UIItemMegaphone.h
    IO/UITypes/UIInviteCard.cpp
    IO/UITypes/UIInviteCard.h
    IO/UITypes/UITradingRoom.cpp
    IO/UITypes/UITradingRoom.h
    IO/UITypes/UIPersonalShop.cpp
    IO/UITypes/UIPersonalShop.h
    IO/UITypes/UIMonsterBook.cpp
    IO/UITypes/UIMonsterBook.h
    IO/UITypes/UISkillBook.cpp
    IO/UITypes/UISkillBook.h
    IO/UITypes/SkillMacroLayout.h
    IO/UITypes/UISkillMacro.cpp
    IO/UITypes/UISkillMacro.h
    IO/UITypes/UISystemOption.cpp
    IO/UITypes/UISystemOption.h
    IO/UITypes/UISoftKey.cpp
    IO/UITypes/UISoftKey.h
    IO/UITypes/UIStatsInfo.cpp
    IO/UITypes/UIStatsInfo.h
    IO/UITypes/UIStatusBar.cpp
    IO/UITypes/UIStatusBar.h
    IO/UITypes/UIStatusMessenger.cpp
    IO/UITypes/UIStatusMessenger.h
    IO/UITypes/UITermsOfService.cpp
    IO/UITypes/UITermsOfService.h
    IO/UITypes/UIUserList.cpp
    IO/UITypes/UIUserList.h
    IO/UITypes/UIWorldMap.cpp
    IO/UITypes/UIWorldMap.h
    IO/UITypes/UIWorldSelect.cpp
    IO/UITypes/UIWorldSelect.h
    IO/Cursor.cpp
    IO/Cursor.h
    IO/KeyAction.h
    IO/Keyboard.cpp
    IO/Keyboard.h
    IO/KeyConfig.h
    IO/KeyType.h
    IO/Messages.cpp
    IO/Messages.h
    IO/BuddyCommands.cpp
    IO/BuddyCommands.h
    IO/PartyCommands.cpp
    IO/PartyCommands.h
    IO/GuildCommands.cpp
    IO/GuildCommands.h
    IO/ChatCommand.h
    IO/ScreenshotRules.h
    IO/UI.cpp
    IO/UI.h
    IO/UIDragElement.h
    IO/UIElement.cpp
    IO/UIElement.h
    IO/UIState.h
    IO/UIStateCashShop.cpp
    IO/UIStateCashShop.h
    IO/UIStateGame.cpp
    IO/UIStateGame.h
    IO/LoginScene.cpp
    IO/LoginScene.h
    IO/UIStateLogin.cpp
    IO/UIStateLogin.h
    IO/Window.cpp
    IO/Window.h
    Net/Handlers/Helpers/CashShopParser.cpp
    Net/Handlers/Helpers/CashShopParser.h
    Net/Handlers/Helpers/CharacterParser.cpp
    Net/Handlers/Helpers/CharacterParser.h
    Net/Handlers/Helpers/ItemParser.cpp
    Net/Handlers/Helpers/ItemParser.h
    Net/Handlers/Helpers/LoginParser.cpp
    Net/Handlers/Helpers/LoginParser.h
    Net/Handlers/Helpers/MovementParser.cpp
    Net/Handlers/Helpers/MovementParser.h
    Net/Handlers/Helpers/PetParser.h
    Net/Handlers/Helpers/BuffParser.cpp
    Net/Handlers/Helpers/BuffParser.h
    Net/Handlers/Helpers/MobStatParser.cpp
    Net/Handlers/Helpers/MobStatParser.h
    Net/Handlers/Helpers/GuildParser.cpp
    Net/Handlers/Helpers/GuildParser.h
    Net/Handlers/AttackHandlers.cpp
    Net/Handlers/AttackHandlers.h
    Net/Handlers/BuddyHandlers.cpp
    Net/Handlers/BuddyHandlers.h
    Net/Handlers/CashShopHandlers.cpp
    Net/Handlers/CashShopHandlers.h
    Net/Handlers/CommonHandlers.cpp
    Net/Handlers/CommonHandlers.h
    Net/Handlers/GuildHandlers.cpp
    Net/Handlers/GuildHandlers.h
    Net/Handlers/InventoryHandlers.cpp
    Net/Handlers/InventoryHandlers.h
    Net/Handlers/LoginHandlers.cpp
    Net/Handlers/LoginHandlers.h
    Net/Handlers/MapObjectHandlers.cpp
    Net/Handlers/MapObjectHandlers.h
    Net/Handlers/MessagingHandlers.cpp
    Net/Handlers/MessagingHandlers.h
    Net/Handlers/NpcInteractionHandlers.cpp
    Net/Handlers/NpcInteractionHandlers.h
    Net/Handlers/PartyHandlers.cpp
    Net/Handlers/PartyHandlers.h
    Net/Handlers/PlayerHandlers.cpp
    Net/Handlers/PlayerHandlers.h
    Net/Handlers/PlayerInteractionHandlers.cpp
    Net/Handlers/PlayerInteractionHandlers.h
    Net/Handlers/SetFieldHandlers.cpp
    Net/Handlers/SetFieldHandlers.h
    Net/Handlers/TestingHandlers.cpp
    Net/Handlers/TestingHandlers.h
    Net/Packets/AttackAndSkillPackets.h
    Net/Packets/CharCreationPackets.h
    Net/Packets/CommonPackets.h
    Net/Packets/GameplayPackets.h
    Net/Packets/InventoryPackets.h
    Net/Packets/LoginPackets.h
    Net/Packets/MessagingPackets.h
    Net/Packets/MiniRoomPackets.h
    Net/Packets/MovementPacket.h
    Net/Packets/NpcInteractionPackets.h
    Net/Packets/QuestPackets.h
    Net/Packets/PlayerPackets.h
    Net/Packets/SelectCharPackets.h
    Net/Cryptography.cpp
    Net/Cryptography.h
    Net/InPacket.cpp
    Net/InPacket.h
    Net/Login.h
    Net/NetConstants.h
    Net/OutPacket.cpp
    Net/OutPacket.h
    Net/PacketError.h
    Net/PacketHandler.h
    Net/PacketSwitch.cpp
    Net/PacketSwitch.h
    Net/FrameAssembler.h
    Net/Session.cpp
    Net/Session.h
    Net/SocketAsio.cpp
    Net/SocketAsio.h
    Net/SocketWinsock.cpp
    Net/SocketWinsock.h
    Template/BoolPair.h
    Template/Cache.h
    Template/Enumeration.h
    Template/EnumMap.h
    Template/Interpolated.h
    Template/Optional.h
    Template/Point.h
    Template/Range.h
    Template/Rectangle.h
    Template/Singleton.h
    Template/TimedQueue.h
    Template/TypeMap.h
    Util/CharacterName.h
    Util/Chalkboard.h
    Util/ChatWrap.h
    Util/CurseWords.h
    Util/HardwareInfo.h
    Util/Lerp.h
    Util/Misc.cpp
    Util/Misc.h
    Util/NpcTextMarkup.h
    Util/SwindleWords.h
    Util/NxFiles.cpp
    Util/NxFiles.h
    Util/QuadTree.h
    Util/Randomizer.h
    Util/ScreenResolution.h
    Util/TimedBool.h
    Configuration.cpp
    Configuration.h
    Constants.h
    Error.h
    MapleStory.cpp
    MapleStory.h
    resource.h
    Timer.h Audio/membuf.cpp Audio/membuf.h
)

TARGET_INCLUDE_DIRECTORIES(HeavenClient PUBLIC ${PROJECT_SOURCE_DIR}/libs/NoLifeNx)
TARGET_LINK_LIBRARIES(HeavenClient ${PROJECT_SOURCE_DIR}/libs/NoLifeNx/nlnx/build/libNoLifeNx.a)

TARGET_LINK_LIBRARIES(HeavenClient ${PROJECT_SOURCE_DIR}/libs/lz4/lib/liblz4.a)

INCLUDE(FindFreetype)
IF(NOT FREETYPE_FOUND)
    FIND_LIBRARY(FREETYPE_LIBRARIES NAMES libfreetype freetype.dll PATHS "./libs/MacOS" "./libs/Windows" DOC "Freetype library")
    FIND_PATH(FREETYPE_INCLUDE_DIRS ftbuild.h "./includes" DOC "Freetype includes")
ENDIF(NOT FREETYPE_FOUND)
INCLUDE_DIRECTORIES(${FREETYPE_INCLUDE_DIRS})
TARGET_LINK_LIBRARIES(HeavenClient ${FREETYPE_LIBRARIES})

TARGET_INCLUDE_DIRECTORIES(HeavenClient PUBLIC ${PROJECT_SOURCE_DIR}/libs/glad/build/include)
TARGET_LINK_LIBRARIES(HeavenClient ${PROJECT_SOURCE_DIR}/libs/glad/build/libglad.a)

TARGET_INCLUDE_DIRECTORIES(HeavenClient PUBLIC ${PROJECT_SOURCE_DIR}/libs/stb)

TARGET_INCLUDE_DIRECTORIES(HeavenClient PUBLIC ${PROJECT_SOURCE_DIR}/libs/asio/asio/include)

TARGET_INCLUDE_DIRECTORIES(HeavenClient PUBLIC ${PROJECT_SOURCE_DIR}/libs/openal-soft/include)
IF(APPLE)
    TARGET_LINK_LIBRARIES(HeavenClient /usr/local/Cellar/openal-soft/1.20.1/lib/libopenal.dylib)
ELSE()
    TARGET_LINK_LIBRARIES(HeavenClient ${PROJECT_SOURCE_DIR}/libs/openal-soft/build/libopenal.so)
ENDIF()

TARGET_INCLUDE_DIRECTORIES(HeavenClient PUBLIC ${PROJECT_SOURCE_DIR}/libs/glfw/include)
IF(APPLE)
    TARGET_LINK_LIBRARIES(HeavenClient /usr/local/Cellar/glfw/3.3.2/lib/libglfw.dylib)
ELSE(APPLE)
    TARGET_LINK_LIBRARIES(HeavenClient ${PROJECT_SOURCE_DIR}/libs/glfw/build/src/libglfw3.a)
ENDIF()

TARGET_INCLUDE_DIRECTORIES(HeavenClient PUBLIC ${PROJECT_SOURCE_DIR}/libs/alure/include)
IF(APPLE)
    TARGET_LINK_LIBRARIES(HeavenClient /usr/local/Cellar/alure/1.2_1/lib/libalure.dylib)
ELSE()
    TARGET_LINK_LIBRARIES(HeavenClient ${PROJECT_SOURCE_DIR}/libs/alure/build/libalure2.so)
ENDIF()

TARGET_LINK_LIBRARIES(HeavenClient GL)
TARGET_LINK_LIBRARIES(HeavenClient X11)
IF(NOT APPLE)
    TARGET_LINK_LIBRARIES(HeavenClient dl)
    TARGET_LINK_LIBRARIES(HeavenClient pthread)
ENDIF()

TARGET_COMPILE_DEFINITIONS(HeavenClient PRIVATE HEAVENCLIENT_SOURCE_DIR="${PROJECT_SOURCE_DIR}")

ADD_CUSTOM_COMMAND(TARGET HeavenClient POST_BUILD
    COMMAND ${CMAKE_COMMAND} -E copy_directory
        ${PROJECT_SOURCE_DIR}/fonts
        $<TARGET_FILE_DIR:HeavenClient>/fonts
    # The quests the server supports; the client offers only these.
    COMMAND ${CMAKE_COMMAND} -E copy
        ${PROJECT_SOURCE_DIR}/../game-data/supported_quests.txt
        $<TARGET_FILE_DIR:HeavenClient>/supported_quests.txt
)
