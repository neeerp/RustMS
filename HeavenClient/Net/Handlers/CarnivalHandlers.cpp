#include "CarnivalHandlers.h"

#include "../../Character/Carnival.h"
#include "../../Gameplay/Stage.h"
#include "../../IO/UI.h"
#include "../../IO/UITypes/UIChatBar.h"
#include "../../IO/UITypes/UIMonsterCarnival.h"
#include "../../Util/Misc.h"

#include <nlnx/nx.hpp>

namespace ms
{
	namespace
	{
		void chat_line(const std::string& line, UIChatBar::LineType type)
		{
			if (auto chatbar = UI::get().get_element<UIChatBar>())
				chatbar->send_chatline(line, type);
		}

		std::string trimmed(std::string name)
		{
			size_t first = name.find_first_not_of(' ');

			return first == std::string::npos ? "" : name.substr(first);
		}

		// What the field's monsterCarnival lists name the `index`th of tab
		// `tab`: a mob, a skill (Skill.nx MCSkill.img) or a guardian
		// (MCGuardian.img).
		std::string item_name(uint8_t tab, uint8_t index)
		{
			nl::node carnival = NxHelper::Map::get_map_node_name(Stage::get().get_mapid())["monsterCarnival"];
			std::string entry = std::to_string(index);

			switch (tab)
			{
			case 0:
			{
				int32_t mob = carnival["mob"][entry]["id"];
				std::string id = std::to_string(mob);

				return trimmed(nl::nx::string["Mob.img"][id]["name"]);
			}
			case 1:
			{
				std::string skill = std::to_string(static_cast<int32_t>(carnival["skill"][entry]));

				return trimmed(nl::nx::skill["MCSkill.img"][skill]["name"]);
			}
			default:
			{
				std::string guardian = std::to_string(static_cast<int32_t>(carnival["guardian"][entry]));

				return trimmed(nl::nx::skill["MCGuardian.img"][guardian]["name"]);
			}
			}
		}

	}

	void CarnivalStartHandler::handle(InPacket& recv) const
	{
		// The revive room's (FUN_00565cad) carries only the side.
		if (carnival::battle_field(carnival::field_type()))
		{
			carnival::start(carnival::read_start(recv));
			UIMonsterCarnival::show();
		}
		else
		{
			carnival::reset();
			carnival::state().team = recv.read_byte();
		}

		Stage::get().get_player().set_team(carnival::state().team);
	}

	void CarnivalObtainedCpHandler::handle(InPacket& recv) const
	{
		carnival::state().mine = carnival::read_points(recv);
	}

	void CarnivalPartyCpHandler::handle(InPacket& recv) const
	{
		int8_t team = recv.read_byte();
		carnival::Points points = carnival::read_points(recv);

		if (team == 0 || team == 1)
			carnival::state().sides[team] = points;
	}

	void CarnivalSummonHandler::handle(InPacket& recv) const
	{
		uint8_t tab = static_cast<uint8_t>(recv.read_byte());
		uint8_t index = static_cast<uint8_t>(recv.read_byte());
		std::string who = recv.read_string();

		std::string what = item_name(tab, index);

		Stage::get().get_player().request_answered();
		chat_line(carnival::summon_line(tab, who, what), UIChatBar::LineType::GREY);

		// A mob counts on the window's side panel too.
		if (tab == 0)
			if (auto window = UI::get().get_element<UIMonsterCarnival>())
				window->summoned(what);
	}

	void CarnivalMessageHandler::handle(InPacket& recv) const
	{
		uint8_t code = static_cast<uint8_t>(recv.read_byte());

		Stage::get().get_player().request_answered();

		if (auto line = carnival::message_line(code))
			chat_line(*line, UIChatBar::LineType::GREY);
	}

	void CarnivalDiedHandler::handle(InPacket& recv) const
	{
		int8_t team = recv.read_byte();
		std::string name = recv.read_string();
		uint8_t cp = static_cast<uint8_t>(recv.read_byte());

		chat_line(carnival::died_line(team, name, cp), UIChatBar::LineType::GREY);
	}

	void CarnivalLeaveHandler::handle(InPacket& recv) const
	{
		// Only the battle field's OnPacket reads it (FUN_00565962); the
		// revive room's doesn't.
		if (!carnival::battle_field(carnival::field_type()))
			return;

		uint8_t type = static_cast<uint8_t>(recv.read_byte());
		int8_t team = recv.read_byte();
		std::string name = recv.read_string();

		chat_line(carnival::leave_line(type, team, name), UIChatBar::LineType::SKILL);
	}

	void CarnivalResultHandler::handle(InPacket& recv) const
	{
		uint8_t code = static_cast<uint8_t>(recv.read_byte());

		if (auto line = carnival::result_line(code))
			chat_line(*line, UIChatBar::LineType::SKILL);
	}
}
