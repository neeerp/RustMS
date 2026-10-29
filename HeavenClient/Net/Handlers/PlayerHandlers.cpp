//////////////////////////////////////////////////////////////////////////////////
//	This file is part of the continued Journey MMORPG client					//
//	Copyright (C) 2015-2019  Daniel Allendorf, Ryan Payton						//
//																				//
//	This program is free software: you can redistribute it and/or modify		//
//	it under the terms of the GNU Affero General Public License as published by	//
//	the Free Software Foundation, either version 3 of the License, or			//
//	(at your option) any later version.											//
//																				//
//	This program is distributed in the hope that it will be useful,				//
//	but WITHOUT ANY WARRANTY; without even the implied warranty of				//
//	MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the				//
//	GNU Affero General Public License for more details.							//
//																				//
//	You should have received a copy of the GNU Affero General Public License	//
//	along with this program.  If not, see <https://www.gnu.org/licenses/>.		//
//////////////////////////////////////////////////////////////////////////////////
#include "PlayerHandlers.h"

#include "Helpers/BuffParser.h"
#include "Helpers/LoginParser.h"
#include "../../Audio/Audio.h"
#include "../../Character/Diseases.h"
#include "../../Character/QuestCatalog.h"
#include "../../Character/MountRules.h"
#include "../../Gameplay/Combat/DojoEnergy.h"
#include "../../Gameplay/Combat/Massacre.h"

#include "../Packets/LoginPackets.h"
#include "../Packets/PlayerPackets.h"

#include "../../Configuration.h"
#include "../../Data/ItemData.h"
#include "../../Data/SkillData.h"
#include "../../Gameplay/Stage.h"
#include "../../Gameplay/MapleMap/GuideRules.h"
#include "../../IO/PartySearchActions.h"
#include "../../IO/UI.h"
#include "../../IO/Window.h"

#include "../../IO/UITypes/UIBuffList.h"
#include "../../IO/UITypes/UICashShop.h"
#include "../../IO/UITypes/UIChatBar.h"
#include "../../IO/UITypes/EnergyBarLayout.h"
#include "../../IO/UITypes/UIEnergyBar.h"
#include "../../IO/UITypes/UIInviteCard.h"
#include "../../IO/UITypes/UIMonsterBook.h"
#include "../../IO/UITypes/UINotice.h"
#include "../../IO/UITypes/UIQuestLog.h"
#include "../../IO/UITypes/UISkillBook.h"
#include "../../IO/UITypes/UIStatsInfo.h"

#include "Timer.h"

namespace ms
{
	namespace
	{
		void transition_channel_change()
		{
			Window::get().fadeout(
				[]()
				{
					GraphicsGL::get().clear();

					UI::get().enable();
					Timer::get().start();
					GraphicsGL::get().unlock();
				}
			);

			GraphicsGL::get().lock();
			Stage::get().clear();
			Timer::get().start();
		}
	}

	void ChangeChannelHandler::handle(InPacket& recv) const
	{
		int32_t character_id = Stage::get().get_player().get_oid();
		// v83 resumes a search in the new channel (0x776020).
		PartySearchActions::leaving_channel();
		transition_channel_change();
		LoginParser::parse_login(recv);

		auto cashshop = UI::get().get_element<UICashShop>();

		if (cashshop)
			cashshop->exit_cashshop();
		else
			PlayerLoginPacket(character_id).dispatch();
	}

	void ChangeStatsHandler::handle(InPacket& recv) const
	{
		// Set when this answers the player's item or stat request.
		if (recv.read_bool())
			Stage::get().get_player().request_answered();

		int32_t updatemask = recv.read_int();

		Player& player = Stage::get().get_player();
		uint16_t oldlevel = player.get_level();
		bool recalculate = false;

		for (auto iter : MapleStat::codes)
		{
			// The pets' serials follow the hair, whichever changed (FUN_004e2fba).
			if (iter.first == MapleStat::Id::PET)
				continue;

			if (updatemask & iter.second)
				recalculate |= handle_stat(iter.first, recv);

			if (iter.first == MapleStat::Id::HAIR)
				for (int32_t bit : { 0x8, 0x80000, 0x100000 })
					if (updatemask & bit)
						recv.skip(8);
		}

		// A new level may allow other ammo.
		if (recalculate || player.get_level() != oldlevel)
			player.recalc_stats();

		// v83 checks Berserk on each HP change only (0xa20276).
		if (updatemask & MapleStat::codes[MapleStat::Id::HP])
			player.update_berserk(false);

		// Then on any change the pet's MP potion, if readied (0xa202a8).
		player.check_auto_mp();

		// After a level-up, v83 tells anyone but a GM to spend AP once more than 199 pile up (FUN_00a1fb52).
		const CharStats& stats = player.get_stats();

		if (player.get_level() > oldlevel && stats.get_stat(MapleStat::Id::AP) > 199
			&& stats.get_job().get_id() % 1000 / 100 != 9)
			UI::get().emplace<UIOk>(
				"If you do not use your AP when you level up, \\nthen you won't be able to receive any additional AP.\\nPlease use your AP.",
				[](bool) {});

		// Then it offers each party quest that opens at the new level; O shows
		// it on the Quest Log's Available tab.
		if (player.get_level() > oldlevel)
			if (auto cards = UI::get().get_element<UIInviteCard>())
				for (const PartyQuest* quest : party_quest_cards(QuestCatalog::get().party_quests(),
					stats.get_job().get_id(), player.get_level()))
					cards->party_quest(*quest, [id = quest->id](bool yes)
					{
						if (!yes)
							return;

						auto log = UI::get().get_element<UIQuestLog>();

						if (!log || !log->is_active())
							UI::get().emplace<UIQuestLog>(Stage::get().get_player().get_quests());

						if (auto opened = UI::get().get_element<UIQuestLog>())
							opened->show_quest(id, 0);
					});

		// The tutorial guide leaves at level 11 (FUN_00a1fb52).
		if (guide_rules::leaves(oldlevel, player.get_level()))
			Stage::get().get_guide().remove();

		// v83 checks the alert's quests again on any stat change (FUN_00a0843c).
		Stage::get().get_quest_alert().stats_changed();

		UI::get().enable();
	}

	bool ChangeStatsHandler::handle_stat(MapleStat::Id stat, InPacket& recv) const
	{
		Player& player = Stage::get().get_player();

		bool recalculate = false;

		switch (stat)
		{
		case MapleStat::Id::SKIN:
			// A byte, as v83 reads it (FUN_004e2fba).
			player.change_look(stat, recv.read_byte());
			break;
		case MapleStat::Id::FACE:
		case MapleStat::Id::HAIR:
			player.change_look(stat, recv.read_int());
			break;
		case MapleStat::Id::LEVEL:
			player.change_level(recv.read_byte());
			break;
		case MapleStat::Id::JOB:
			player.change_job(recv.read_short());
			break;
		case MapleStat::Id::EXP:
			player.get_stats().set_exp(recv.read_int());
			break;
		case MapleStat::Id::MESO:
			player.get_inventory().set_meso(recv.read_int());
			break;
		case MapleStat::Id::GACHAEXP:
			// An int, as v83 reads it (FUN_004e2fba).
			player.get_stats().set_gachaexp(recv.read_int());
			break;
		default:
			player.get_stats().set_stat(stat, recv.read_short());
			recalculate = true;
			break;
		}

		bool update_statsinfo = need_statsinfo_update(stat);

		if (update_statsinfo && !recalculate)
			if (auto statsinfo = UI::get().get_element<UIStatsInfo>())
				statsinfo->update_stat(stat);

		bool update_skillbook = need_skillbook_update(stat);

		if (update_skillbook)
		{
			int16_t value = player.get_stats().get_stat(stat);

			if (auto skillbook = UI::get().get_element<UISkillBook>())
				skillbook->update_stat(stat, value);
		}

		return recalculate;
	}

	bool ChangeStatsHandler::need_statsinfo_update(MapleStat::Id stat) const
	{
		switch (stat)
		{
		case MapleStat::Id::JOB:
		case MapleStat::Id::STR:
		case MapleStat::Id::DEX:
		case MapleStat::Id::INT:
		case MapleStat::Id::LUK:
		case MapleStat::Id::HP:
		case MapleStat::Id::MAXHP:
		case MapleStat::Id::MP:
		case MapleStat::Id::MAXMP:
		case MapleStat::Id::AP:
		case MapleStat::Id::EXP:
			return true;
		default:
			return false;
		}
	}

	bool ChangeStatsHandler::need_skillbook_update(MapleStat::Id stat) const
	{
		switch (stat)
		{
		case MapleStat::Id::JOB:
		case MapleStat::Id::SP:
			return true;
		default:
			return false;
		}
	}

	void BuffHandler::handle(InPacket& recv) const
	{
		BuffParser::Flags flags = BuffParser::read_flags(recv);

		for (uint8_t bit : BuffParser::given(flags))
			handle_buff(recv, Buffstat::by_bit(bit));

		if (flags.has(BuffParser::ENERGY) || flags.has(BuffParser::DASH_SPEED) || flags.has(BuffParser::DASH_JUMP) ||
			flags.has(BuffParser::RIDING) || flags.has(BuffParser::INFUSION) || flags.has(BuffParser::GUIDED))
			handle_two_state(recv, flags);

		Stage::get().get_player().recalc_stats();
		// Giving and cancelling buffs check the alert's quests again too (FUN_00a202be, FUN_00a2071f).
		Stage::get().get_quest_alert().stats_changed();
	}

	void ApplyBuffHandler::handle_buff(InPacket& recv, Buffstat::Id bs) const
	{
		int16_t value = recv.read_short();
		int32_t skillid = recv.read_int();
		int32_t duration = recv.read_int();

		if (bs == Buffstat::NONE)
			return;

		bs = disease::given(bs, skillid);
		Stage::get().get_player().give_buff({ bs, value, skillid, duration });

		// A mob's disease has no icon, only a notice of what it does
		// (FUN_00932da6). The icon follows the reason alone (0xa203bf), so
		// one a skill gave, as Dragon Roar stuns its user, has the skill's.
		if (disease::is_disease(bs))
		{
			std::string notice = disease::notice(disease::skill(skillid));

			if (!notice.empty())
				if (auto chatbar = UI::get().get_element<UIChatBar>())
					chatbar->send_chatline(notice, UIChatBar::LineType::GREY);

			if (skillid <= 0 || SkillData::get(skillid).get_name().empty())
				return;
		}

		if (auto bufflist = UI::get().get_element<UIBuffList>())
			bufflist->add_buff(skillid, bs, duration);
	}

	// The player's energy changed, they dashed, or they got on a mount (0xa202be). The two-state stats' data
	// follows the defense attack and state.
	void ApplyBuffHandler::handle_two_state(InPacket& recv, const BuffParser::Flags& flags) const
	{
		recv.skip(2);

		BuffParser::TwoState state = BuffParser::read_two_state(recv, flags);

		// An energy of 0 takes the gauge down (0x95f065).
		if (flags.has(BuffParser::ENERGY))
		{
			UIEnergyBar::show(state.energy);
			Stage::get().get_player().show_energy(state.energy_skill, state.energy >= energy_bar::FULL, false);
			Stage::get().get_player().set_energy(state.energy);
		}

		// Dash lasts until the server ends it; its time is the icon's alone.
		if (flags.has(BuffParser::DASH_SPEED))
		{
			BuffParser::Dash dash = state.dash;
			int32_t duration = dash.seconds * 1000;
			Player& player = Stage::get().get_player();
			player.give_buff({ Buffstat::DASH_SPEED, static_cast<int16_t>(dash.speed), dash.skill, duration });
			player.give_buff({ Buffstat::DASH_JUMP, static_cast<int16_t>(dash.jump), dash.skill, duration });
			player.show_dash(dash.skill);

			// Space Dash shows no icon.
			if (auto bufflist = UI::get().get_element<UIBuffList>(); bufflist && !mount::is_space_dash(dash.skill))
				bufflist->add_buff(dash.skill, Buffstat::DASH_SPEED, duration);
		}

		if (flags.has(BuffParser::INFUSION))
		{
			BuffParser::Infusion infusion = state.infusion;
			int32_t duration = infusion.seconds * 1000;
			Stage::get().get_player().give_buff(
				{ Buffstat::SPEED_INFUSION, static_cast<int16_t>(infusion.speed), infusion.skill, duration });

			if (auto bufflist = UI::get().get_element<UIBuffList>())
				bufflist->add_buff(infusion.skill, Buffstat::SPEED_INFUSION, duration);
		}

		// Homing Beacon marks its mob, with no icon (0x7b24ee).
		if (flags.has(BuffParser::GUIDED))
			Stage::get().get_player().set_beacon({ state.beacon.skill, state.beacon.mob });

		if (!flags.has(BuffParser::RIDING))
			return;

		BuffParser::Ride ride = state.ride;
		Stage::get().get_player().change_ride(ride.vehicle);

		if (ride.skill <= 0)
			return;

		if (auto bufflist = UI::get().get_element<UIBuffList>())
			bufflist->add_buff(ride.skill, Buffstat::MONSTER_RIDING, INT32_MAX);
	}

	// The player's energy ran out, their Dash ended, or they got off (0xa2071f).
	void CancelBuffHandler::handle_two_state(InPacket&, const BuffParser::Flags& flags) const
	{
		if (flags.has(BuffParser::ENERGY))
		{
			UIEnergyBar::show(0);
			Stage::get().get_player().show_energy(0, false, false);
			Stage::get().get_player().set_energy(0);
		}

		if (flags.has(BuffParser::DASH_SPEED))
		{
			Player& player = Stage::get().get_player();
			player.cancel_buff(Buffstat::DASH_SPEED);
			player.cancel_buff(Buffstat::DASH_JUMP);
			player.show_dash(0);

			if (auto bufflist = UI::get().get_element<UIBuffList>())
				bufflist->cancel_buff(Buffstat::DASH_SPEED);
		}

		if (flags.has(BuffParser::INFUSION))
		{
			Stage::get().get_player().cancel_buff(Buffstat::SPEED_INFUSION);

			if (auto bufflist = UI::get().get_element<UIBuffList>())
				bufflist->cancel_buff(Buffstat::SPEED_INFUSION);
		}

		if (flags.has(BuffParser::GUIDED))
			Stage::get().get_player().set_beacon({});

		if (!flags.has(BuffParser::RIDING))
			return;

		Stage::get().get_player().change_ride(0);

		if (auto bufflist = UI::get().get_element<UIBuffList>())
			bufflist->cancel_buff(Buffstat::MONSTER_RIDING);
	}

	void CancelBuffHandler::handle_buff(InPacket&, Buffstat::Id bs) const
	{
		if (bs == Buffstat::NONE)
			return;

		Stage::get().get_player().cancel_buff(bs);

		// Undead ends with Puppet's bit.
		if (bs == Buffstat::PUPPET)
			Stage::get().get_player().cancel_buff(Buffstat::UNDEAD);

		if (auto bufflist = UI::get().get_element<UIBuffList>())
			bufflist->cancel_buff(bs);
	}

	void RecalculateStatsHandler::handle(InPacket&) const
	{
		Stage::get().get_player().recalc_stats();
	}

	void SetTamingMobInfoHandler::handle(InPacket& recv) const
	{
		int32_t cid = recv.read_int();
		int32_t level = recv.read_int();
		int32_t exp = recv.read_int();
		int32_t fatigue = recv.read_int();
		bool levelup = recv.read_bool();

		Char *character = nullptr;

		if (Stage::get().is_player(cid))
			character = &Stage::get().get_player();
		else if (Optional<OtherChar> other = Stage::get().get_chars().get_char(cid))
			character = other.get();

		if (!character)
			return;

		character->set_mount_info(level, exp, fatigue);

		if (levelup)
			character->show_mount_levelup();
	}

	void UpdateSkillHandler::handle(InPacket& recv) const
	{
		// Set when this answers the player's SP request.
		if (recv.read_bool())
			Stage::get().get_player().request_answered();

		// v83 FUN_00a1e48c takes several, as SP Reset sends.
		int16_t count = recv.read_short();

		for (int16_t i = 0; i < count; i++)
		{
			int32_t skillid = recv.read_int();
			int32_t level = recv.read_int();
			int32_t masterlevel = recv.read_int();
			int64_t expire = recv.read_long();

			Stage::get().get_player().change_skill(skillid, level, masterlevel, expire);

			if (auto skillbook = UI::get().get_element<UISkillBook>())
				skillbook->update_skills(skillid);
		}

		// So does a skill update (FUN_00a1e48c).
		Stage::get().get_quest_alert().stats_changed();

		UI::get().enable();
	}

	void SkillUseResultHandler::handle(InPacket& recv) const
	{
		recv.skip_byte();

		Stage::get().get_player().request_answered();
	}

	void SkillLearnItemResultHandler::handle(InPacket& recv) const
	{
		int32_t cid = recv.read_int();
		auto character = Stage::get().get_character(cid);

		if (!character)
			return;

		bool mastery = recv.read_bool();
		recv.skip_int();	// skill
		recv.skip_int();	// master level
		bool used = recv.read_bool();
		bool success = recv.read_bool();

		if (used)
			character->show_skill_book(success);

		if (!Stage::get().is_player(cid))
			return;

		Stage::get().get_player().request_answered();
		Sound(success ? Sound::Name::ENCHANTSUCCESS : Sound::Name::ENCHANTFAILURE).play();

		std::string book = mastery ? "Mastery Book" : "Skill Book";
		std::string message;

		if (!used)
			message = "You cannot use " + book + ".";
		else if (!success)
			message = "Despite using " + book + ", the effect was nowhere to be found.";
		else if (mastery)
			message = "The Book of Mastery glows brightly, and the current skills have gone through an upgrade.";
		else
			message = "The Skill Book glows brightly, and new skills have now been added.";

		if (auto chatbar = UI::get().get_element<UIChatBar>())
			chatbar->send_chatline(message, UIChatBar::LineType::SKILL);
	}

	void SkillMacrosHandler::handle(InPacket& recv) const
	{
		// v83 reads at most five and leaves the rest (FUN_004e77b0).
		size_t count = std::min<size_t>(recv.read_byte(), SkillMacros::MAX_MACROS);
		std::vector<SkillMacro> macros(count);

		for (SkillMacro& macro : macros)
		{
			macro.name = recv.read_string();
			macro.silent = recv.read_byte() != 0;

			for (int32_t& skill : macro.skills)
				skill = recv.read_int();
		}

		UI::get().get_skill_macros().replace(std::move(macros));
		Stage::get().stop_macro();
	}

	void QuickSlotInitHandler::handle(InPacket& recv) const
	{
		QuickSlots::Keys keys = QuickSlots::DEFAULTS;

		if (recv.read_byte() != 0)
			for (int32_t& key : keys)
				key = recv.read_int();

		UI::get().get_quick_slots().init(keys);

		// v83 sends every layout it is given straight back.
		QuickSlotPacket(keys).dispatch();
	}

	void AddCooldownHandler::handle(InPacket& recv) const
	{
		int32_t skill_id = recv.read_int();
		int16_t cooltime = recv.read_short();

		// The Battleship's HP comes as a cooldown of its own, not one to wait out (decomp.c:1169755).
		if (skill_id == mount::BATTLESHIP_HP)
		{
			auto hp = static_cast<uint16_t>(cooltime);
			bool shipped = mount::is_ship(Stage::get().get_player().get_vehicle());

			if (hp == 0 || shipped)
				if (auto bufflist = UI::get().get_element<UIBuffList>())
					bufflist->set_gauge(mount::BATTLESHIP, hp);

			return;
		}

		Stage::get().get_player().add_cooldown(skill_id, cooltime);
	}

	void KeymapHandler::handle(InPacket& recv) const
	{
		recv.skip(1);

		for (uint8_t i = 0; i < 90; i++)
		{
			uint8_t type = recv.read_byte();
			int32_t action = recv.read_int();

			UI::get().add_keymapping(i, type, action);
		}
	}

	void AutoHpPotHandler::handle(InPacket& recv) const
	{
		Stage::get().get_player().set_auto_potion(false, recv.read_int());
	}

	void AutoMpPotHandler::handle(InPacket& recv) const
	{
		Stage::get().get_player().set_auto_potion(true, recv.read_int());
	}

	void CheckAutoHpHandler::handle(InPacket& recv) const
	{
		Stage::get().get_player().check_auto_hp(recv.read_int());
	}

	void ItemEmotionHandler::handle(InPacket& recv) const
	{
		Stage::get().use_item_emotion(recv.read_int());
	}

	void MonsterBookSetCardHandler::handle(InPacket& recv) const
	{
		// v83 writes both lines in chat type 0xc and reads nothing more for a full card (FUN_00a081b8).
		auto chatbar = UI::get().get_element<UIChatBar>();

		if (!recv.read_bool())
		{
			if (chatbar)
				chatbar->send_chatline("This card is already full in the Monster Book. This card will disappear.",
									   UIChatBar::LineType::SKILL);

			return;
		}

		int32_t card = recv.read_int();
		int32_t level = recv.read_int();

		Stage::get().get_player().get_monsterbook().set_card(card, level);

		if (chatbar)
		{
			const std::string &name = ItemData::get(card).get_name();

			chatbar->send_chatline("[" + name + "] has been successfully recorded on the Monster Book.",
								   UIChatBar::LineType::SKILL);
		}

		if (auto monsterbook = UI::get().get_element<UIMonsterBook>())
			monsterbook->update_cards();
	}

	void MonsterBookSetCoverHandler::handle(InPacket& recv) const
	{
		int32_t card = recv.read_int();

		Stage::get().get_player().get_monsterbook().set_cover(card);

		if (auto monsterbook = UI::get().get_element<UIMonsterBook>())
			monsterbook->update_cards();
	}

	void DojoWarpUpHandler::handle(InPacket& recv) const
	{
		if (recv.read_byte() != 0)
			Stage::get().get_player().clear_request();

		Stage::get().teleport_to_portal(recv.read_byte());
	}

	void SessionValueHandler::handle(InPacket& recv) const
	{
		std::string key = recv.read_string();
		std::string value = recv.read_string();

		if (key == "energy")
			dojo_energy::session().energy = std::atoi(value.c_str());
		else
			massacre::store(massacre::session(), key, std::atoi(value.c_str()));
	}

	void FieldSetVariableHandler::handle(InPacket& recv) const
	{
		std::string key = recv.read_string();
		std::string value = recv.read_string();

		if (key == "Bamboo_Used")
			dojo_energy::session().bamboo_used = !value.empty() && value != "0";
	}
}
