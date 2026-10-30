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

#include "CoconutHandlers.h"
#include "SnowballHandlers.h"
#include "Timer.h"

#include "Helpers/CharacterParser.h"
#include "Helpers/LoginParser.h"

#include "../Packets/GameplayPackets.h"

#include "../../Audio/Audio.h"
#include "../../Character/Carnival.h"
#include "../../Character/Inventory/ItemExpiry.h"
#include "../../Character/Ariant.h"
#include "../../Character/Coconut.h"
#include "../../Configuration.h"

#include "../../Gameplay/FieldEffectRules.h"
#include "../../Gameplay/Combat/DojoEnergy.h"
#include "../../Gameplay/Combat/Massacre.h"
#include "../../Gameplay/OxQuizRules.h"
#include "../../Gameplay/Stage.h"
#include "../../IO/BuddyCommands.h"
#include "../../IO/MarriageActions.h"
#include "../../IO/PartySearchActions.h"
#include "../../IO/UI.h"
#include "../../IO/UiControl.h"
#include "../../IO/UITypes/UIChatBar.h"
#include "../../IO/UITypes/UINotice.h"
#include "../../IO/Window.h"
#include "../../Util/Memos.h"
#include "../../Util/Misc.h"

#include "../../IO/UITypes/UICharSelect.h"
#include "../../IO/UITypes/UIAriant.h"
#include "../../IO/UITypes/UICoconut.h"
#include "../../IO/UITypes/UIDojoHud.h"
#include "../../IO/UITypes/UIFieldClock.h"
#include "../../IO/UITypes/UIMemoNotice.h"
#include "../../IO/UITypes/UIMiniMap.h"
#include "../../IO/UITypes/UIMobGage.h"
#include "../../IO/UITypes/UIPyramidHud.h"
#include "../../IO/UITypes/UIPyramidResult.h"
#include "../../IO/UITypes/UIStatsInfo.h"
#include "../../IO/UITypes/UITradingRoom.h"
#include "../../IO/UITypes/UIMiniGame.h"
#include "../../IO/UITypes/UITimedQuests.h"

#include <nlnx/nx.hpp>

#include <cstdlib>
#include <optional>

namespace ms
{
	namespace
	{
		std::optional<CharEntry> cached_player_entry;

		// SET_FIELD ends with the server's time; v83 keeps how far it is
		// from ours (0x529366).
		void server_time(InPacket& recv)
		{
			if (recv.length() >= 8)
				item_expiry::server_offset() = recv.read_long() - item_expiry::now();
		}
	}

	void SetFieldHandler::transition(int32_t mapid, uint8_t portalid, std::optional<Point<int16_t>> chase) const
	{
		int32_t fieldtype = NxHelper::Map::get_map_node_name(mapid)["info"]["fieldType"];
		carnival::enter_field(mapid, fieldtype);
		UI::get().remove(UIElement::Type::MONSTERCARNIVAL);
		enter_coconut_field(mapid, fieldtype);
		enter_snowball_field(mapid, fieldtype);
		UI::get().remove(UIElement::Type::COCONUT);
		ariant::enter_field(fieldtype);
		UI::get().remove(UIElement::Type::ARIANT);
		massacre::enter_field(massacre::session(), fieldtype);
		UI::get().remove(UIElement::Type::PYRAMIDHUD);
		UI::get().remove(UIElement::Type::PYRAMIDRESULT);

		Window::get().fadeout(
			[mapid, portalid, chase]()
			{
				GraphicsGL::get().clear();

				Stage::get().load(mapid, portalid, chase);

				// The Dojo's HUD is up only where its secret skills may be used.
				UIDojoHud::show_on(Stage::get().get_mapinfo().get_fieldtype());
				// And Coconut Season's scoreboard on its own.
				UICoconut::show_on(Stage::get().get_mapinfo().get_fieldtype());
				// And the Battle Arena's board.
				UIAriant::show_on(Stage::get().get_mapinfo().get_fieldtype());
				// And a Pyramid stage's.
				UIPyramidHud::show_on(Stage::get().get_mapinfo().get_fieldtype());
				UIPyramidResult::show_waiting();

				UI::get().enable();
				Timer::get().start();
				GraphicsGL::get().unlock();

				Stage::get().transfer_player();

				// Requests left unanswered come back (FUN_00a3fecf).
				BuddyCommands::show_requests();

				// So do notes left unread (0x5293eb).
				if (!Memos::Inbox::get().empty() && !UIMemoNotice::showing())
					UI::get().emplace<UIMemoNotice>();
			});

		// Bamboo Rain is once a stage, and the Dojo's HUD and its clock go with their field.
		dojo_energy::session().bamboo_used = false;
		dojo_energy::session().clock = {};
		UI::get().remove(UIElement::Type::DOJOHUD);

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

		server_time(recv);

		// Respawning after death restores some HP.
		Stage::get().get_player().get_stats().set_stat(MapleStat::Id::HP, hp);

		// The stat window keeps its text until told; the status bar reads HP
		// every frame.
		if (auto statsinfo = UI::get().get_element<UIStatsInfo>())
			statsinfo->update_stat(MapleStat::Id::HP);

		// A request still waiting is forgotten in the new field, and so are
		// the potions the pets lacked (FUN_00948b7b).
		Stage::get().get_player().clear_request();
		Stage::get().get_player().forget_potion_misses();

		// The Monster Carnival's sides are the battle's; START sets them
		// again in its fields.
		Stage::get().get_player().set_team(carnival::NO_TEAM);

		// The server ends a trade when we leave the field.
		if (auto room = UI::get().get_element<UITradingRoom>())
			room->close();

		// And a game, which the other player wins.
		if (auto game = UI::get().get_element<UIMiniGame>())
			game->close();

		// The field's countdown and boss HP bar go with it.
		UI::get().remove(UIElement::Type::FIELDCLOCK);

		// So does Duey's window, and the server isn't told (report b029).
		UI::get().remove(UIElement::Type::DUEY);

		// And the wedding's wishlist and gift windows: the server forgets the
		// open window when we leave the field.
		UI::get().remove(UIElement::Type::WISHLISTINPUT);
		UI::get().remove(UIElement::Type::WEDDINGREGISTRY);

		// A line being typed in the strip is dropped with its field; the
		// expanded log keeps both, and LOCK_UI leaves the bar be (0xa04156).
		if (auto chatbar = UI::get().get_element<UIChatBar>())
			if (!ui_control::state().locked && !chatbar->is_chatopen() && chatbar->is_chatfieldopen())
				chatbar->toggle_chatfield(false);

		if (auto gage = UI::get().get_element<UIMobGage>())
			gage->hide();

		// A search ends with any map change in the channel (0xa03f2e).
		PartySearchActions::field_entered();

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

			// A new login has no quest clocks up; a channel change keeps them (FUN_00777347).
			timed_quests::shown().clear();
		}
		else if (cached_player_entry && cached_player_entry->id == cid)
		{
			playerentry = &*cached_player_entry;
		}
		else
		{
			return;
		}

		// The server forgets a proposal on a channel change, the Cash Shop
		// and a new login, without answering it.
		MarriageActions::answered();

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

		// The Dojo's energy starts over with the character, as do the Pyramid's counts.
		dojo_energy::session() = {};
		massacre::session() = {};

		// The server lists this character's notes, if it has any.
		Memos::Inbox::get().listed({});

		Player& player = Stage::get().get_player();

		player.get_buddies().set_capacity(static_cast<uint8_t>(recv.read_byte()));

		// The name Blessing of the Fairy's tooltip shows (0x4e59e7).
		player.set_linked_name(recv.read_bool() ? recv.read_string() : "");

		CharacterParser::parse_inventory(recv, player.get_inventory());
		CharacterParser::parse_skillbook(recv, player.get_skills());
		CharacterParser::parse_cooldowns(recv, player);
		CharacterParser::parse_questlog(recv, player.get_quests());
		CharacterParser::parse_minigame(recv);
		CharacterParser::parse_ring1(recv, player);
		CharacterParser::parse_ring2(recv, player);
		CharacterParser::parse_ring3(recv, player);
		CharacterParser::parse_teleportrock(recv, player.get_teleport_rock());
		CharacterParser::parse_monsterbook(recv, player.get_monsterbook());
		CharacterParser::parse_nyinfo(recv);
		CharacterParser::parse_areainfo(recv, player.get_quests());
		// The character data ends with a short read nowhere here.
		recv.skip(2);
		server_time(recv);

		player.recalc_stats();
		// Logging in or changing channel starts Berserk over (0x7763a9).
		player.update_berserk(true);

		uint8_t portalid = player.get_stats().get_portal();
		int32_t mapid = player.get_stats().get_mapid();

		transition(mapid, portalid);

		UI::get().change_state(UI::State::GAME);

		if (!timed_quests::shown().empty())
			UI::get().emplace<UITimedQuests>();

		// After a channel change or the Cash Shop, a search goes on. Its
		// indicator belongs to the new game state.
		PartySearchActions::field_entered();
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

		// Coconut Season's clock is its scoreboard's, whatever the kind (0x5498c6).
		if (coconut::on_field())
		{
			coconut::state().end = coconut::clock_end(coconut::now(), recv.read_int());

			return;
		}

		// A Pyramid stage heeds only kind 2, which sets its own clock (0x560389).
		if (massacre::session().field == massacre::FIELD_STAGE)
		{
			if (kind == 2)
				massacre::start_clock(massacre::session(), UIDojoHud::now(), recv.read_int());

			return;
		}

		// A Dojo field heeds only kind 2, which sets its HUD's clock (0x55522b).
		if (UI::get().get_element<UIDojoHud>())
		{
			if (kind == 2)
				dojo_energy::session().clock.start(UIDojoHud::now(), recv.read_int());

			return;
		}

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
			{
				int32_t seconds = recv.read_int();

				// Kept for the Dojo's HUD as well, in case the field loading turns out to be one of its own.
				dojo_energy::session().clock.start(UIDojoHud::now(), seconds);
				UI::get().emplace<UIFieldClock>(seconds, false);
				break;
			}
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

	void OxQuizHandler::handle(InPacket& recv) const
	{
		uint8_t flag = recv.read_byte();
		uint8_t set = recv.read_byte();
		uint16_t number = recv.read_short();

		nl::node quiz = nl::nx::etc["OXQuiz.img"];
		nl::node questions = quiz[std::to_string(set)];
		nl::node question = questions[std::to_string(number)];

		ox_quiz::Show show =
			ox_quiz::show(flag, number, static_cast<bool>(quiz), static_cast<bool>(questions), static_cast<bool>(question));

		switch (show)
		{
			case ox_quiz::Show::CLEAR:
				Stage::get().set_notice("");
				break;
			case ox_quiz::Show::KEEP:
				break;
			case ox_quiz::Show::QUESTION:
			{
				Stage::get().set_notice(question["q"].get_string());

				auto event_clock = shown_clock<UIEventClock>();

				if (!event_clock)
					event_clock = UI::get().emplace<UIEventClock>(ox_quiz::CLOCK_SECONDS);

				if (event_clock)
					event_clock->start();

				break;
			}
			case ox_quiz::Show::ANSWER:
				Stage::get().set_notice(ox_quiz::answer_line(question["d"].get_string(), static_cast<int32_t>(question["a"].get_integer())));
				break;
		}
	}

	void GmEventInstructionsHandler::handle(InPacket& recv) const
	{
		Stage::get().show_event_help(recv.read_byte());
	}

	void StopClockHandler::handle(InPacket&) const
	{
		UI::get().remove(UIElement::Type::FIELDCLOCK);
	}

	namespace
	{
		// A boss site's closing time: a byte, whether the boss is out, then
		// the minutes left, 0 once it has closed. A negative count shows
		// nothing. The lines are v83's string pool entries for each site.
		void closing_line(InPacket& recv, const std::string& summon, const std::string& left,
						  const char* closed)
		{
			bool summoned = recv.read_byte() != 0;
			int32_t minutes = recv.read_int();

			if (minutes < 0)
				return;

			std::string line = minutes == 0
				? std::string(closed)
				: (summoned ? left : summon) + std::to_string(minutes) + " minutes.";

			if (auto chatbar = UI::get().get_element<UIChatBar>())
				chatbar->send_chatline(line, UIChatBar::LineType::SKILL);
		}
	}

	// SP 0x1059, 0x105a, 0x105b.
	void HorntailCaveHandler::handle(InPacket& recv) const
	{
		closing_line(recv, "The Horntail's Cave will close if you do not summon Horntail in ",
					 "The Horntail's Cave will close in ", "The Horntail's Cave has closed.");
	}

	// SP 0x1056, 0x1057, 0x1058.
	void ZakumShrineHandler::handle(InPacket& recv) const
	{
		closing_line(recv, "The Zakum Shrine will close if you do not summon Zakum in ",
					 "The Zakum Shrine will close in ", "The Zakum Shrine has closed.");
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

	void BlowWeatherHandler::handle(InPacket& recv) const
	{
		uint8_t flag = recv.read_byte();
		int32_t item_id = recv.read_int();
		// Only a start by a player's item carries their message.
		std::string msg = item_id != 0 && flag == 0 ? recv.read_string() : "";

		Stage::get().set_weather_flag(flag);
		Stage::get().blow_weather(item_id, msg, false);
	}

	void PlayJukeboxHandler::handle(InPacket& recv) const
	{
		int32_t item_id = recv.read_int();

		if (!field_effect::is_jukebox(item_id))
			return;

		std::string id = std::to_string(item_id);
		std::string msg = nl::nx::string["Cash.img"][id]["msg"];
		std::string line = field_effect::jukebox_line(msg, recv.read_string());

		if (auto chatbar = UI::get().get_element<UIChatBar>())
			chatbar->send_chatline(line, UIChatBar::LineType::LIGHT_YELLOW);

		std::string path = nl::nx::item["Cash"]["0510.img"]["0" + id]["info"]["path"];

		Music(field_effect::jukebox_path(path))
			.play(field_effect::JUKEBOX_FADE, field_effect::JUKEBOX_DELAY, field_effect::JUKEBOX_FADE_IN);
		Stage::get().start_jukebox();
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
