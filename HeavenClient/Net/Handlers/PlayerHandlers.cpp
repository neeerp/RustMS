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

#include "../Packets/LoginPackets.h"

#include "../../Configuration.h"
#include "../../Data/ItemData.h"
#include "../../Gameplay/Stage.h"
#include "../../IO/UI.h"
#include "../../IO/Window.h"

#include "../../IO/UITypes/UIBuffList.h"
#include "../../IO/UITypes/UICashShop.h"
#include "../../IO/UITypes/UIChatBar.h"
#include "../../IO/UITypes/UIMonsterBook.h"
#include "../../IO/UITypes/UINotice.h"
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
			if (updatemask & iter.second)
				recalculate |= handle_stat(iter.first, recv);

		if (recalculate)
			player.recalc_stats(false);

		// After a level-up, v83 tells anyone but a GM to spend AP once more than 199 pile up (FUN_00a1fb52).
		const CharStats& stats = player.get_stats();

		if (player.get_level() > oldlevel && stats.get_stat(MapleStat::Id::AP) > 199
			&& stats.get_job().get_id() % 1000 / 100 != 9)
			UI::get().emplace<UIOk>(
				"If you do not use your AP when you level up, \\nthen you won't be able to receive any additional AP.\\nPlease use your AP.",
				[](bool) {});

		UI::get().enable();
	}

	bool ChangeStatsHandler::handle_stat(MapleStat::Id stat, InPacket& recv) const
	{
		Player& player = Stage::get().get_player();

		bool recalculate = false;

		switch (stat)
		{
		case MapleStat::Id::SKIN:
			player.change_look(stat, recv.read_short());
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

		Stage::get().get_player().recalc_stats(false);
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

		// A disease has no icon, only a notice of what it does
		// (FUN_00932da6).
		if (disease::is_disease(bs))
		{
			std::string notice = disease::notice(disease::skill(skillid));

			if (!notice.empty())
				if (auto chatbar = UI::get().get_element<UIChatBar>())
					chatbar->send_chatline(notice, UIChatBar::LineType::GREY);

			return;
		}

		if (auto bufflist = UI::get().get_element<UIBuffList>())
			bufflist->add_buff(skillid, bs, duration);
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
		Stage::get().get_player().recalc_stats(false);
	}

	void UpdateSkillHandler::handle(InPacket& recv) const
	{
		// Set when this answers the player's SP request.
		if (recv.read_bool())
			Stage::get().get_player().request_answered();

		recv.skip(2);

		int32_t skillid = recv.read_int();
		int32_t level = recv.read_int();
		int32_t masterlevel = recv.read_int();
		int64_t expire = recv.read_long();

		Stage::get().get_player().change_skill(skillid, level, masterlevel, expire);

		if (auto skillbook = UI::get().get_element<UISkillBook>())
			skillbook->update_skills(skillid);

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
		uint8_t size = recv.read_byte();

		for (uint8_t i = 0; i < size; i++)
		{
			recv.read_string();	// name
			recv.read_byte();	// 'shout' byte
			recv.read_int();	// skill 1
			recv.read_int();	// skill 2
			recv.read_int();	// skill 3
		}
	}

	void AddCooldownHandler::handle(InPacket& recv) const
	{
		int32_t skill_id = recv.read_int();
		int16_t cooltime = recv.read_short();

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
}
