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
#include "SetFieldHandlers.h"

#include "Timer.h"

#include "Helpers/CharacterParser.h"
#include "Helpers/LoginParser.h"

#include "../Packets/GameplayPackets.h"

#include "../../Configuration.h"

#include "../../Gameplay/Stage.h"
#include "../../IO/BuddyCommands.h"
#include "../../IO/UI.h"
#include "../../IO/UITypes/UIChatBar.h"
#include "../../IO/UITypes/UINotice.h"
#include "../../IO/Window.h"

#include "../../IO/UITypes/UICharSelect.h"
#include "../../IO/UITypes/UIFieldClock.h"
#include "../../IO/UITypes/UIMiniMap.h"
#include "../../IO/UITypes/UIMobGage.h"
#include "../../IO/UITypes/UIStatsInfo.h"
#include "../../IO/UITypes/UITradingRoom.h"

#include <cstdlib>
#include <optional>

namespace ms
{
	namespace
	{
		std::optional<CharEntry> cached_player_entry;
	}

	void SetFieldHandler::transition(int32_t mapid, uint8_t portalid, std::optional<Point<int16_t>> chase) const
	{
		Window::get().fadeout(
			[mapid, portalid, chase]()
			{
				GraphicsGL::get().clear();

				Stage::get().load(mapid, portalid, chase);

				UI::get().enable();
				Timer::get().start();
				GraphicsGL::get().unlock();

				Stage::get().transfer_player();

				// Requests left unanswered come back (FUN_00a3fecf).
				BuddyCommands::show_requests();
			});

		GraphicsGL::get().lock();
		Stage::get().clear();
		Timer::get().start();
	}

	void SetFieldHandler::handle(InPacket& recv) const
	{
		Constants::Constants::get().set_viewwidth(Setting<Width>::get().load());
		Constants::Constants::get().set_viewheight(Setting<Height>::get().load());

		int32_t channel = recv.read_int();
		Configuration::get().set_channelid(static_cast<uint8_t>(channel));

		// v83 (FUN_00776020) gives the new field the first byte as its key.
		// The second says whether the character's data follows.
		Stage::get().set_field_key(recv.read_byte());
		bool chardata = recv.read_bool();

		if (chardata)
			set_field(recv);
		else
			change_map(recv, channel);
	}

	void SetFieldHandler::change_map(InPacket& recv, int32_t) const
	{
		recv.skip(3);

		int32_t mapid = recv.read_int();
		int8_t portalid = recv.read_byte();
		int16_t hp = recv.read_short();

		// v83 (FUN_00776020) puts the player at a point instead of the
		// portal when chased there, as back through a Mystic Door.
		std::optional<Point<int16_t>> chase;

		if (recv.length() > 0 && recv.read_bool())
		{
			auto x = static_cast<int16_t>(recv.read_int());
			auto y = static_cast<int16_t>(recv.read_int());
			chase = Point<int16_t>(x, y);
		}

		// Respawning after death restores some HP.
		Stage::get().get_player().get_stats().set_stat(MapleStat::Id::HP, hp);

		// The stat window keeps its text until told; the status bar reads HP
		// every frame.
		if (auto statsinfo = UI::get().get_element<UIStatsInfo>())
			statsinfo->update_stat(MapleStat::Id::HP);

		// A request still waiting is forgotten in the new field.
		Stage::get().get_player().clear_request();

		// The server ends a trade when we leave the field.
		if (auto room = UI::get().get_element<UITradingRoom>())
			room->close();

		// The field's countdown and boss HP bar go with it.
		UI::get().remove(UIElement::Type::FIELDCLOCK);

		if (auto gage = UI::get().get_element<UIMobGage>())
			gage->hide();

		// v83 (FUN_00a03eba, from CField::OnSetField) sends this on every
		// field change, not only on entering the game.
		PlayerUpdatePacket().dispatch();

		transition(mapid, portalid, chase);
	}

	void SetFieldHandler::set_field(InPacket& recv) const
	{
		recv.skip(23);

		int32_t cid = recv.read_int();
		auto charselect = UI::get().get_element<UICharSelect>();
		const CharEntry* playerentry = nullptr;

		if (charselect)
		{
			const CharEntry& selected = charselect->get_character(cid);

			if (selected.id != cid)
				return;

			cached_player_entry = selected;
			playerentry = &*cached_player_entry;
		}
		else if (cached_player_entry && cached_player_entry->id == cid)
		{
			playerentry = &*cached_player_entry;
		}
		else
		{
			return;
		}

		// The cached entry is the character-select snapshot; after a channel
		// change its map and stats are stale, so take them from this packet.
		// Only the character list carries rankings, so keep those.
		CharEntry entry = *playerentry;
		StatsEntry stats = LoginParser::parse_stats(recv);
		stats.rank = entry.stats.rank;
		stats.jobrank = entry.stats.jobrank;
		entry.stats = stats;
		cached_player_entry = entry;

		Stage::get().loadplayer(entry);

		Player& player = Stage::get().get_player();

		player.get_buddies().set_capacity(static_cast<uint8_t>(recv.read_byte()));

		if (recv.read_bool())
			recv.read_string(); // 'linkedname'

		CharacterParser::parse_inventory(recv, player.get_inventory());
		CharacterParser::parse_skillbook(recv, player.get_skills());
		CharacterParser::parse_cooldowns(recv, player);
		CharacterParser::parse_questlog(recv, player.get_quests());
		CharacterParser::parse_minigame(recv);
		CharacterParser::parse_ring1(recv);
		CharacterParser::parse_ring2(recv);
		CharacterParser::parse_ring3(recv);
		CharacterParser::parse_teleportrock(recv, player.get_teleport_rock());
		CharacterParser::parse_monsterbook(recv, player.get_monsterbook());
		CharacterParser::parse_nyinfo(recv);
		CharacterParser::parse_areainfo(recv);

		player.recalc_stats(true);

		PlayerUpdatePacket().dispatch();

		uint8_t portalid = player.get_stats().get_portal();
		int32_t mapid = player.get_stats().get_mapid();

		transition(mapid, portalid);

		UI::get().change_state(UI::State::GAME);
	}

	namespace
	{
		void blocked_line(const char *message)
		{
			if (auto chatbar = UI::get().get_element<UIChatBar>())
				chatbar->send_chatline(message, UIChatBar::LineType::SKILL);
		}
	}

	void BlockedMapHandler::handle(InPacket& recv) const
	{
		auto &player = Stage::get().get_player();
		player.request_answered();

		// HeavenClient notes the portal's map when it asks; it stays put.
		player.get_stats().set_mapid(Stage::get().get_mapid());

		switch (recv.read_byte())
		{
			case 1:
				blocked_line("The portal is closed for now.");
				break;
			case 2:
				blocked_line("You cannot go to that place.");
				break;
			case 3:
			case 5:
				blocked_line("Unable to approach due to the force of the ground.");
				break;
			case 4:
				UI::get().emplace<UIOk>("You cannot teleport to or on this map.", [](bool) {});
				break;
			case 6:
				blocked_line("This map can only be entered by party members.");
				break;
			case 7:
				UI::get().emplace<UIOk>("The cash shop is currently\\nnot available.\\nStay tuned...", [](bool) {});
				break;
			default:
				break;
		}
	}

	void BlockedServerHandler::handle(InPacket& recv) const
	{
		Stage::get().get_player().request_answered();

		switch (recv.read_byte())
		{
			case 1:
				blocked_line("You cannot move that channel. Please try again later.");
				break;
			case 2:
				blocked_line("You cannot go into the cash shop. Please try again later.");
				break;
			case 3:
				blocked_line("The Item-Trading Shop is currently unavailable. Please try again later.");
				break;
			case 4:
				blocked_line("You cannot go into the trade shop, due to the limitation of user count.");
				break;
			case 5:
				blocked_line("You do not meet the minimum level requirement to access the Trade Shop.");
				break;
			default:
				break;
		}
	}

	namespace
	{
		template<class T>
		Optional<T> shown_clock()
		{
			auto clock = UI::get().get_element<T>();

			return clock && clock->is_active() ? clock : Optional<T>();
		}
	}

	void ClockHandler::handle(InPacket& recv) const
	{
		int8_t kind = recv.read_byte();
		auto field_clock = shown_clock<UIFieldClock>();

		// Only kind 3 keeps the field's clock its own.
		if (kind != 3 && field_clock)
			field_clock->unswitch();

		switch (kind)
		{
			case 0:
			{
				// An event's clock: a time stands until a 0 or less starts
				// it, and does not reset one that is up (0xa24c82).
				int32_t seconds = recv.read_int();
				auto event_clock = shown_clock<UIEventClock>();

				if (seconds != 0 && !event_clock)
					event_clock = UI::get().emplace<UIEventClock>(std::abs(seconds));

				if (seconds <= 0 && event_clock)
					event_clock->start();

				break;
			}
			case 1:
			{
				uint8_t hour = recv.read_byte();
				uint8_t minute = recv.read_byte();
				uint8_t second = recv.read_byte();
				Stage::get().set_clock(hour, minute, second);
				break;
			}
			case 2:
				UI::get().emplace<UIFieldClock>(recv.read_int(), false);
				break;
			case 3:
				// Takes down a clock it put up, and puts one up if asked and
				// there is none.
				if (field_clock && field_clock->is_switched())
				{
					UI::get().remove(UIElement::Type::FIELDCLOCK);
					field_clock = {};
				}

				if (!field_clock && recv.read_bool())
					UI::get().emplace<UIFieldClock>(recv.read_int(), true);

				break;
			default:
				break;
		}
	}

	void StopClockHandler::handle(InPacket&) const
	{
		UI::get().remove(UIElement::Type::FIELDCLOCK);
	}

	void ObjectStateHandler::handle(InPacket& recv) const
	{
		std::string name = recv.read_string();
		int32_t state = recv.read_int();
		Stage::get().set_object_state(name, state);
	}

	void ObjectStateListHandler::handle(InPacket& recv) const
	{
		int32_t count = recv.read_int();

		for (int32_t i = 0; i < count; i++)
		{
			std::string name = recv.read_string();
			int32_t state = recv.read_int();
			Stage::get().set_object_state(name, state);
		}
	}

	void MapObjectVisibleHandler::handle(InPacket& recv) const
	{
		uint8_t count = recv.read_byte();

		for (uint8_t i = 0; i < count; i++)
		{
			std::string tag = recv.read_string();
			bool shown = recv.read_bool();
			Stage::get().show_tag(tag, shown);
		}
	}

	void BackEffectHandler::handle(InPacket& recv) const
	{
		uint8_t mode = recv.read_byte();
		// Read but unused (FUN_00587d24).
		recv.skip_int();
		uint8_t index = recv.read_byte();
		int32_t duration = recv.read_int();
		Stage::get().fade_back(mode, index, duration);
	}

	void ClearBackEffectHandler::handle(InPacket&) const
	{
		Stage::get().reset_backs();
	}

	void MiniMapOnOffHandler::handle(InPacket& recv) const
	{
		// Any non-zero byte is on.
		UIMiniMap::switch_on(recv.read_byte() != 0);
	}

	void ObstacleResetHandler::handle(InPacket&) const
	{
		Stage::get().reset_obstacles();
	}

	void ContiMoveHandler::handle(InPacket& recv) const
	{
		uint8_t target = recv.read_byte();
		uint8_t event = recv.read_byte();
		Stage::get().move_ship(target, event);
	}

	void ContiStateHandler::handle(InPacket& recv) const
	{
		uint8_t state = recv.read_byte();
		uint8_t flag = recv.read_byte();
		Stage::get().show_ship_state(state, flag);
	}
}
