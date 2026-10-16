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
#include "Stage.h"

#include "../Configuration.h"
#include "QuestConversation.h"
#include "Timer.h"
#include "MapleMap/CharClick.h"
#include "MapleMap/Npc.h"

#include "../IO/UI.h"
#include "../Util/Misc.h"
#include "../Util/NxFiles.h"

#include "../IO/UITypes/UINpcTalk.h"
#include "../IO/UITypes/UIStatusBar.h"
#include "../Net/Packets/AttackAndSkillPackets.h"
#include "../Net/Packets/GameplayPackets.h"

#include <nlnx/nx.hpp>

namespace ms
{
	namespace
	{
		// An in-map portal's move lands 120 ms after Up; the portal then
		// waits 600 ms (v83 FUN_00957b74, FUN_0094e5e5).
		constexpr int16_t TELEPORT_DELAY = 120;
		constexpr int16_t TELEPORT_COOLDOWN = 600;
	}

	Stage::Stage() : combat(player, chars, mobs, reactors)
	{
		state = State::INACTIVE;
	}

	void Stage::init()
	{
		drops.init();
	}

	void Stage::load(int32_t mapid, int8_t portalid)
	{
		switch (state)
		{
			case State::INACTIVE:
				load_map(mapid);
				respawn(portalid);
				break;
			case State::TRANSITION:
				respawn(portalid);
				break;
		}

		state = State::ACTIVE;
	}

	void Stage::loadplayer(const CharEntry &entry)
	{
		player = entry;
		playable = player;

		start = ContinuousTimer::get().start();

		CharStats stats = player.get_stats();
		levelBefore = stats.get_stat(MapleStat::Id::LEVEL);
		expBefore = stats.get_exp();
	}

	void Stage::clear()
	{
		state = State::INACTIVE;

		chars.clear();
		npcs.clear();
		mobs.clear();
		drops.clear();
		reactors.clear();
	}

	void Stage::load_map(int32_t mapid)
	{
		Stage::mapid = mapid;

		std::string strid = string_format::extend_id(mapid, 9);
		std::string prefix = std::to_string(mapid / 100000000);

		nl::node src = mapid == -1 ? NxFiles::ui_classic["CashShopPreview.img"] : nl::nx::map["Map"]["Map" + prefix][strid + ".img"];

		tilesobjs = MapTilesObjs(src);
		backgrounds = MapBackgrounds(src["back"]);
		physics = Physics(src["foothold"], src["info"]);
		mapinfo = MapInfo(src, physics.get_fht().get_walls(), physics.get_fht().get_borders());
		portals = MapPortals(src["portal"], mapid);
	}

	void Stage::respawn(int8_t portalid)
	{
		// v83's Cash Shop stage has no music of its own; the Cash Shop plays
		// its theme.
		Music(mapid == -1 ? "BgmUI.img/ShopBgm" : mapinfo.get_bgm()).play();

		Point<int16_t> spawnpoint = portals.get_portal_by_id(portalid);
		Point<int16_t> startpos = physics.get_y_below(spawnpoint);
		player.respawn(startpos, mapinfo.is_underwater());
		player.set_recovery_rate(mapinfo.get_recovery());
		left_ladder = nullptr;
		teleport_delay = 0;
		teleport_cooldown = 0;
		camera.set_view(mapinfo.get_walls(), mapinfo.get_borders());
		camera.set_position(startpos);
	}

	void Stage::draw(float alpha) const
	{
		if (state != State::ACTIVE)
			return;

		Point<int16_t> viewpos = camera.position(alpha);
		Point<double> viewrpos = camera.realposition(alpha);
		double viewx = viewrpos.x();
		double viewy = viewrpos.y();

		backgrounds.drawbackgrounds(viewx, viewy, alpha);

		for (auto id : Layer::IDs)
		{
			tilesobjs.draw(id, viewpos, alpha);
			reactors.draw(id, viewx, viewy, alpha);
			npcs.draw(id, viewx, viewy, alpha);
			mobs.draw(id, viewx, viewy, alpha);
			chars.draw(id, viewx, viewy, alpha);
			player.draw(id, viewx, viewy, alpha);
			drops.draw(id, viewx, viewy, alpha);
		}

		const Party &party = player.get_party();

		if (party.shows_hp_marks())
			chars.draw_party_hp(party, player.get_oid(), mapid, viewx, viewy, alpha);

		combat.draw(viewx, viewy, alpha);
		portals.draw(viewpos, alpha);
		backgrounds.drawforegrounds(viewx, viewy, alpha);
		effect.draw();
	}

	void Stage::update()
	{
		if (state != State::ACTIVE)
			return;

		combat.update();
		backgrounds.update();
		effect.update();
		tilesobjs.update();

		reactors.update(physics);
		npcs.update(physics);
		mobs.update(physics, player.get_position(), player.is_dead());
		chars.update(physics);
		drops.update(physics);
		player.update(physics);
		check_held_ladders();
		update_teleport();

		portals.update(player.get_position());
		check_touch_portals();
		camera.update(player.get_position());

		if (player.is_invincible())
			return;

		if (int32_t oid_id = mobs.find_colliding(player.get_phobj()))
		{
			if (MobAttack attack = mobs.create_attack(oid_id))
			{
				MobAttackResult result = player.damage(attack);
				TakeDamagePacket(result, TakeDamagePacket::From::TOUCH).dispatch();
			}
		}
	}

	void Stage::show_character_effect(int32_t cid, CharEffect::Id effect)
	{
		if (auto character = get_character(cid))
			character->show_effect_id(effect);
	}

	void Stage::check_portals()
	{
		if (player.is_attacking())
			return;

		Point<int16_t> playerpos = player.get_position();
		const Portal *portal = portals.find_up_portal(playerpos);

		if (!portal)
			return;

		Portal::WarpInfo warpinfo = portal->getwarpinfo();

		if (warpinfo.valid)
		{
			enter_portal(warpinfo);

			return;
		}

		// A script portal with nowhere to lead runs its script on the server
		// (v83 FUN_0094df9b).
		switch (portal->get_type())
		{
			case Portal::SCRIPTED:
			case Portal::SCRIPTED_INVISIBLE:
			case Portal::SCRIPTED_HIDDEN:
				player.request(ChangeMapSpecialPacket(portal->get_name(), playerpos), ExclRequest::WAIT);
				break;
			default:
				break;
		}
	}

	// v83 checks touch portals every update, whatever the player is doing
	// (FUN_0094dac6, called from CUserLocal::Update).
	void Stage::check_touch_portals()
	{
		Point<int16_t> playerpos = player.get_position();
		const Portal *portal = portals.find_touch_portal(playerpos);

		if (!portal)
			return;

		switch (portal->get_type())
		{
			case Portal::TOUCH:
				if (portal->getwarpinfo().valid)
					enter_portal(portal->getwarpinfo());
				break;
			case Portal::SCRIPTED_TOUCH:
				// The script runs again whenever the player may send a request
				// and the portal's delay has passed, unless it runs only once.
				if (portals.may_run_script(*portal) &&
					player.request(ChangeMapSpecialPacket(portal->get_name(), playerpos), portal->get_delay()))
					portals.ran_script(*portal);
				break;
			default:
				break;
		}
	}

	void Stage::enter_portal(const Portal::WarpInfo &warpinfo)
	{
		if (warpinfo.intramap)
		{
			// v83 ignores the portal while a move is pending and for a
			// while after one.
			if (teleport_delay > 0 || teleport_cooldown > 0)
				return;

			Point<int16_t> spawnpoint = portals.get_portal_by_name(warpinfo.toname);
			teleport_target = physics.get_y_below(spawnpoint);
			teleport_delay = TELEPORT_DELAY;

			player.show_effect_id(CharEffect::Id::TELEPORT);
			Sound(Sound::Name::PORTAL).play();
		} else if (warpinfo.valid)
		{
			// v83 asks to change map only when no request waits on the server
			// and 500 ms have passed since the last one, dead or alive
			// (FUN_0053035d).
			if (!player.request(ChangeMapPacket(warpinfo.name, player.get_position()), ExclRequest::WAIT, true))
				return;

			player.get_stats().set_mapid(warpinfo.mapid);

			Sound(Sound::Name::PORTAL).play();
		}
	}

	void Stage::update_teleport()
	{
		if (teleport_cooldown > 0)
			teleport_cooldown -= Constants::TIMESTEP;

		if (teleport_delay <= 0)
			return;

		teleport_delay -= Constants::TIMESTEP;

		if (teleport_delay <= 0)
		{
			player.respawn(teleport_target, mapinfo.is_underwater());
			left_ladder = nullptr;
			teleport_cooldown = TELEPORT_COOLDOWN;
		}
	}

	void Stage::check_seats()
	{
		if (player.is_sitting() || player.is_attacking())
			return;

		Optional<const Seat> seat = mapinfo.findseat(player.get_position());
		player.set_seat(seat);
	}

	void Stage::check_ladders(bool up)
	{
		if (player.is_climbing() || player.is_attacking())
			return;

		Optional<const Ladder> ladder = mapinfo.findladder(player.get_position(), up);
		player.set_ladder(ladder);
	}

	// Holding Up catches any ladder or rope the player reaches, as pressing
	// it there does. The one they just climbed off waits until they land, so
	// jumping off it sideways with Up still held does not catch it again.
	void Stage::check_held_ladders()
	{
		if (player.is_climbing())
		{
			left_ladder = player.get_ladder();
			return;
		}

		if (player.get_phobj().onground)
			left_ladder = nullptr;

		if (!player.is_key_down(KeyAction::Id::UP))
			return;

		Optional<const Ladder> ladder = mapinfo.findladder(player.get_position(), true);

		if (ladder && ladder.get() != left_ladder.get())
			check_ladders(true);
	}

	void Stage::check_drops()
	{
		// The v83 client looks for a drop only when it may ask for one.
		if (!player.can_request(ExclRequest::PICKUP_WAIT))
			return;

		Point<int16_t> playerpos = player.get_position();
		MapDrops::Loot loot = drops.find_loot_at(playerpos, player.get_oid(), player.get_party().get_id());

		if (loot.first)
			player.request(PickupItemPacket(loot.first, loot.second), ExclRequest::PICKUP_WAIT);
	}

	void Stage::send_key(KeyType::Id type, int32_t action, bool down)
	{
		if (state != State::ACTIVE || !playable || player.is_dead())
			return;

		switch (type)
		{
			case KeyType::Id::ACTION:
				if (down)
				{
					switch (action)
					{
						case KeyAction::Id::UP:
							check_ladders(true);
							check_portals();
							break;
						case KeyAction::Id::DOWN:
							check_ladders(false);
							break;
						case KeyAction::Id::SIT:
							check_seats();
							break;
						case KeyAction::Id::ATTACK:
							combat.use_move(0);
							break;
						case KeyAction::Id::PICKUP:
							check_drops();
							break;
						case KeyAction::Id::INTERACT_HARVEST:
							talk_to_nearest_npc();
							break;
					}
				}

				playable->send_action(KeyAction::actionbyid(action), down);
				break;
			case KeyType::Id::SKILL:
				combat.use_move(action);
				break;
			case KeyType::Id::ITEM:
				player.use_item(action);
				break;
			case KeyType::Id::FACE:
				if (down)
				{
					player.set_expression(action);
					// v83 numbers emotes from F1's action (100) as 1.
					FaceExpressionPacket(action - 99).dispatch();
				}

				break;
		}
	}

	void Stage::talk_to_nearest_npc()
	{
		if (state != State::ACTIVE || !playable || player.is_dead())
			return;

		if (auto npctalk = UI::get().get_element<UINpcTalk>(); npctalk && npctalk->is_active())
			return;

		if (Npc *npc = npcs.find_nearest(player.get_position()))
			QuestConversation::talk_to_npc(npc->get_npcid(), npc->get_oid());
	}

	int32_t Stage::character_at(Point<int16_t> cursorpos)
	{
		if (state != State::ACTIVE)
			return 0;

		Point<int16_t> viewpos = camera.position();

		// Others first: the player is often standing in front of someone.
		for (auto &mmo : *chars.get_chars())
		{
			MapObject *character = mmo.second.get();

			if (character->is_active() && char_click::hits(character->get_position() + viewpos, cursorpos))
				return character->get_oid();
		}

		if (char_click::hits(player.get_position() + viewpos, cursorpos))
			return player.get_oid();

		return 0;
	}

	void Stage::release_key(KeyType::Id type, int32_t action)
	{
		if (state == State::ACTIVE && type == KeyType::Id::ACTION)
			player.release_key(KeyAction::actionbyid(action));
	}

	void Stage::release_keys()
	{
		if (state == State::ACTIVE)
			player.release_keys();
	}

	Cursor::State Stage::send_cursor(bool pressed, Point<int16_t> position)
	{
		auto statusbar = UI::get().get_element<UIStatusBar>();

		if (statusbar && statusbar->is_menu_active())
		{
			if (pressed)
				statusbar->remove_menus();

			if (statusbar->is_in_range(position))
				return statusbar->send_cursor(pressed, position);
		}

		return npcs.send_cursor(pressed, position, camera.position());
	}

	bool Stage::is_player(int32_t cid) const
	{
		return cid == player.get_oid();
	}

	MapNpcs &Stage::get_npcs()
	{
		return npcs;
	}

	MapChars &Stage::get_chars()
	{
		return chars;
	}

	MapMobs &Stage::get_mobs()
	{
		return mobs;
	}

	MapReactors &Stage::get_reactors()
	{
		return reactors;
	}

	MapDrops &Stage::get_drops()
	{
		return drops;
	}

	Player &Stage::get_player()
	{
		return player;
	}

	Combat &Stage::get_combat()
	{
		return combat;
	}

	Optional<Char> Stage::get_character(int32_t cid)
	{
		if (is_player(cid))
			return player;
		else
			return chars.get_char(cid);
	}

	int Stage::get_mapid()
	{
		return mapid;
	}

	void Stage::add_effect(std::string path)
	{
		effect = MapEffect(path);
	}

	int64_t Stage::get_uptime()
	{
		return ContinuousTimer::get().stop(start);
	}

	uint16_t Stage::get_uplevel()
	{
		return levelBefore;
	}

	int64_t Stage::get_upexp()
	{
		return expBefore;
	}

	void Stage::transfer_player()
	{
		PlayerMapTransferPacket().dispatch();

		if (Configuration::get().get_admin())
			AdminEnterMapPacket(AdminEnterMapPacket::Operation::ALERT_ADMINS).dispatch();
	}
}