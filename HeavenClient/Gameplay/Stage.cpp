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
#include "MapleMap/DoorRules.h"
#include "MapleMap/Npc.h"

#include "../IO/UI.h"
#include "../Util/Misc.h"
#include "../Util/NxFiles.h"

#include "../IO/UITypes/UIChatBar.h"
#include "../IO/UITypes/UINotice.h"
#include "../IO/UITypes/UINpcTalk.h"
#include "../IO/UITypes/UIPersonalShop.h"
#include "../IO/UITypes/UIStatusBar.h"
#include "../IO/UITypes/UITradingRoom.h"
#include "../Net/Packets/AttackAndSkillPackets.h"
#include "../Net/Packets/GameplayPackets.h"

#include <nlnx/nx.hpp>

namespace ms
{
	namespace
	{
		// An in-map portal's move lands 120 ms after Up; the portal then
		// waits 600 ms (v83 FUN_00957b74, FUN_0094e5e5).
		constexpr int16_t TELEPORT_COOLDOWN = 600;
	}

	Stage::Stage() : combat(player, chars, mobs, reactors, summons, physics, mapinfo, portals)
	{
		state = State::INACTIVE;
	}

	void Stage::init()
	{
		drops.init();

		jump_effect = nl::nx::effect["BasicEff.img"]["VerticalJump"];
		fear_sight = nl::nx::effect["MapEff.img"]["Viewrange"]["3"];
	}

	void Stage::load(int32_t mapid, int8_t portalid, std::optional<Point<int16_t>> chase)
	{
		switch (state)
		{
			case State::INACTIVE:
				load_map(mapid);
				respawn(portalid, chase);

				if (pending_clock && clock.is_present())
					clock.set(pending_clock->hour, pending_clock->minute, pending_clock->second);

				pending_clock.reset();

				// The player's chalkboard follows them, but not to a map where
				// shops open (FUN_00949513).
				if (player.get_chalkboard() && mapinfo.allows_personal_shop())
				{
					player.set_chalkboard(std::nullopt, true);

					if (auto chatbar = UI::get().get_element<UIChatBar>())
						chatbar->send_chatline("You cannot use the chalkboard here.", UIChatBar::LineType::SKILL);
				}

				// A ship field asks where its ship is (FUN_0054db7a).
				if (ship.get_kind() >= 0)
					ContiStateRequestPacket(mapid, static_cast<uint8_t>(ship.get_kind())).dispatch();

				break;
			case State::TRANSITION:
				respawn(portalid, chase);
				break;
		}

		state = State::ACTIVE;
	}

	void Stage::loadplayer(const CharEntry &entry)
	{
		player = entry;
		playable = player;

		start = ContinuousTimer::get().start();
		emote_cooldown = {};
		fame_cooldown = {};

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
		mists.clear();
		doors.clear();
		summons.clear();
		// The server brings the player's pets out again on the new field.
		player.clear_pets();
		pending_clock.reset();
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
		ship = MapShip(src);
		clock = MapClock(src);
		ground_effects = EffectLayer();

		if (mapid != -1)
			player.get_stats().set_mapid(mapid);
	}

	void Stage::respawn(int8_t portalid, std::optional<Point<int16_t>> chase)
	{
		// v83's Cash Shop stage has no music of its own; the Cash Shop plays
		// its theme.
		Music(mapid == -1 ? "BgmUI.img/ShopBgm" : mapinfo.get_bgm()).play();

		// A chased point is on the ground already, as a door is; look from
		// just above it, since its rounded y may sit below a slope.
		Point<int16_t> spawnpoint = chase ? *chase - Point<int16_t>(0, 5) : portals.get_portal_by_id(portalid);
		Point<int16_t> startpos = physics.get_y_below(spawnpoint);
		combat.clear_prepare();
		combat.clear_grenades();
		player.respawn(startpos, mapinfo.is_underwater());
		player.set_recovery_rate(mapinfo.get_recovery());
		pending_teleport.reset();
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
		ship.draw_behind(viewx, viewy, alpha);

		std::vector<MapDoors::Spot> town_doors = get_town_doors();

		for (auto id : Layer::IDs)
		{
			tilesobjs.draw(id, viewpos, alpha);
			doors.draw(id, town_doors, viewpos, alpha);
			reactors.draw(id, viewx, viewy, alpha);
			npcs.draw(id, viewx, viewy, alpha);
			mobs.draw(id, viewx, viewy, alpha);
			chars.draw(id, viewx, viewy, alpha);
			player.draw(id, viewx, viewy, alpha);
			drops.draw(id, viewx, viewy, alpha);
			summons.draw(id, viewx, viewy, alpha);
		}

		summons.draw(-1, viewx, viewy, alpha);
		clock.draw(viewx, viewy);
		ship.draw_above(viewx, viewy, alpha);

		const Party &party = player.get_party();

		if (party.shows_hp_marks())
			chars.draw_party_hp(party, player.get_oid(), mapid, viewx, viewy, alpha);

		// v83 puts a mist's clouds on the effect layer, above the map's
		// objects (z 0xc00614a4).
		mists.draw(viewx, viewy, alpha);
		combat.draw(viewx, viewy, alpha);
		ground_effects.drawabove(viewpos, alpha);
		portals.draw(viewpos, alpha);
		backgrounds.drawforegrounds(viewx, viewy, alpha);
		draw_fear(viewx, viewy, alpha);
		effect.draw();
	}

	void Stage::draw_fear(double viewx, double viewy, float alpha) const
	{
		if (!player.has_buff(Buffstat::Id::FEAR))
			return;

		// v83 copies the sight onto a black layer with its top left this
		// far from the player's feet, ignoring its origin.
		Point<int16_t> sight = player.get_phobj().get_absolute(viewx, viewy, alpha) + Point<int16_t>(-80, -120);
		Point<int16_t> size = fear_sight.get_dimensions();
		int16_t width = Constants::Constants::get().get_viewwidth();
		int16_t height = Constants::Constants::get().get_viewheight();
		int16_t left = sight.x();
		int16_t top = sight.y();
		int16_t right = left + size.x();
		int16_t bottom = top + size.y();

		GraphicsGL &gl = GraphicsGL::get();
		gl.drawrectangle(0, 0, width, top, 0.0f, 0.0f, 0.0f, 1.0f);
		gl.drawrectangle(0, bottom, width, height - bottom, 0.0f, 0.0f, 0.0f, 1.0f);
		gl.drawrectangle(0, top, left, size.y(), 0.0f, 0.0f, 0.0f, 1.0f);
		gl.drawrectangle(right, top, width - right, size.y(), 0.0f, 0.0f, 0.0f, 1.0f);
		fear_sight.draw(DrawArgument(sight + fear_sight.get_origin(), Color::cutout(1.0f)));
	}

	void Stage::update()
	{
		if (state != State::ACTIVE)
			return;

		combat.update();
		backgrounds.update();
		ship.update();
		clock.update();
		effect.update();
		ground_effects.update();
		tilesobjs.update();

		reactors.update(physics);
		mists.update();
		doors.update(physics);
		npcs.update(physics);
		Point<int16_t> playerpos = player.get_position();
		mob_attack::Box playerbody = mob_attack::user_body(playerpos.x(), playerpos.y(), player.is_prone(),
														   !player.getflip());

		mobs.update(physics, { player.get_oid(), playerpos, playerbody, player.get_phobj().onground,
							   player.is_dead(), player.has_buff(Buffstat::Id::DARKSIGHT) });
		chars.update(physics);
		drops.update(physics);

		// v83 tests touch portals and jump pads against where the player is
		// drawn, between its last two 30 ms physics steps (FUN_009b6205), so
		// a player landing is still tested above the floor. The nearest here
		// is where this step started, unless the player was moved there.
		Point<int16_t> drawn = player.get_position();

		Point<int16_t> before = drawn;
		player.update(physics);
		catch_ladders(before);

		if (update_teleport())
			drawn = player.get_position();

		// The player's summons follow where the player now is.
		summons.update(physics, player.get_position(), player.is_climbing());

		portals.update(player.get_position());
		check_touch_portals(drawn);
		camera.update(player.get_position());

		// The player's or a party member's smoke screen keeps mobs' attacks,
		// touch and mists off them (FUN_0043193a).
		bool sheltered = mists.shelters(player.get_position(), [&](int32_t owner) {
			return owner == player.get_oid() || player.get_party().find(owner);
		});

		if (since_mist_hurt <= mist::HURT_INTERVAL)
			since_mist_hurt += Constants::TIMESTEP;

		// Mob attacks land on the player where the mob's update found them,
		// Dark Sight or not; the first takes the player's invincibility.
		for (Mob::Landed &landed : mobs.take_attacks())
		{
			if (player.is_dead() || player.Char::is_invincible() || sheltered)
				break;

			MobAttackResult result = player.damage(landed.attack);
			TakeDamagePacket(result).dispatch();
			combat.interrupt_prepare(result.damage);

			if (result.damage > 0)
				player.show_attack_effect(landed.hit, 1);
		}

		if (player.is_dead() || player.Char::is_invincible() || sheltered)
			return;

		int32_t touching = player.is_invincible() ? 0 : mobs.find_colliding(player.get_phobj());

		if (touching)
		{
			if (MobAttack attack = mobs.create_attack(touching))
			{
				MobAttackResult result = player.damage(attack);
				TakeDamagePacket(result).dispatch();
				combat.interrupt_prepare(result.damage);
			}
		}
		else if (since_mist_hurt > mist::HURT_INTERVAL)
		{
			// Not touching a mob, the player in a mob's mist hurts
			// themselves its x, without a knockback or invincibility
			// (FUN_00431783 at 0x94b7ba).
			mist::Hurt hurt;

			if (mists.hurt_at(player.get_position(), hurt))
			{
				player.show_damage(hurt.damage);
				TakeDamagePacket(hurt.element, hurt.damage, hurt.code).dispatch();
				since_mist_hurt = 0;
			}
		}

		// Stop Motion hurts a player who stood still two seconds.
		if (int32_t damage = player.take_stop_motion())
		{
			player.show_damage(damage);

			if (auto chatbar = UI::get().get_element<UIChatBar>())
				chatbar->send_chatline(disease::STOP_MOTION_HURT, UIChatBar::LineType::GREY);

			TakeDamagePacket(4, damage, 0, 3).dispatch();
		}
	}

	void Stage::show_character_effect(int32_t cid, CharEffect::Id effect)
	{
		if (auto character = get_character(cid))
			character->show_effect_id(effect);
	}

	void Stage::check_portals()
	{
		// v83 takes a portal only while the player stands on a foothold, not
		// in the air or on a ladder (FUN_0094df9b). Holding Up repeats the key,
		// so a player who lands on a portal with Up held goes through.
		if (player.is_attacking() || !player.get_phobj().onground || player.is_climbing())
			return;

		Point<int16_t> playerpos = player.get_position();
		const Portal *portal = portals.find_up_portal(playerpos);

		if (portal)
		{
			Portal::WarpInfo warpinfo = portal->getwarpinfo();

			if (warpinfo.valid)
			{
				enter_portal(warpinfo);

				return;
			}

			// A script portal with nowhere to lead runs its script on the
			// server (v83 FUN_0094df9b).
			switch (portal->get_type())
			{
				case Portal::SCRIPTED:
				case Portal::SCRIPTED_INVISIBLE:
				case Portal::SCRIPTED_HIDDEN:
					player.request(ChangeMapSpecialPacket(field_key, portal->get_name(), playerpos), ExclRequest::WAIT);
					return;
				default:
					break;
			}
		}

		// A town's door portals lead nowhere themselves; a Mystic Door
		// stands on them.
		check_doors(playerpos);
	}

	void Stage::check_doors(Point<int16_t> playerpos)
	{
		if (!mapinfo.is_town())
		{
			if (int32_t owner = doors.find(playerpos))
				player.request(UseDoorPacket(owner, false), ExclRequest::WAIT);

			return;
		}

		for (const door_rules::TownDoor &door : door_rules::town_doors(mapid, player.get_oid(), player.get_door(),
																	   player.get_party()))
		{
			std::optional<Point<int16_t>> point = portals.get_door_portal(door.portal);

			if (point && door_rules::reaches(MapDoors::place(physics, *point).position, playerpos))
			{
				player.request(UseDoorPacket(door.owner, true), ExclRequest::WAIT);

				return;
			}
		}
	}

	std::vector<MapDoors::Spot> Stage::get_town_doors() const
	{
		std::vector<MapDoors::Spot> spots;

		for (const door_rules::TownDoor &door : door_rules::town_doors(mapid, player.get_oid(), player.get_door(),
																	   player.get_party()))
			if (std::optional<Point<int16_t>> point = portals.get_door_portal(door.portal))
				spots.push_back(MapDoors::place(physics, *point));

		return spots;
	}

	// v83 checks touch portals every update, whatever the player is doing
	// (FUN_0094dac6, called from CUserLocal::Update).
	void Stage::check_touch_portals(Point<int16_t> drawn)
	{
		Point<int16_t> playerpos = player.get_position();
		const Portal *portal = portals.find_touch_portal(drawn);

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
					player.request(ChangeMapSpecialPacket(field_key, portal->get_name(), playerpos), portal->get_delay()))
					portals.ran_script(*portal);
				break;
			case Portal::SPRING1:
			case Portal::SPRING2:
				use_jump_pad(*portal);
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
			if (pending_teleport || teleport_cooldown > 0)
				return;

			Point<int16_t> spawnpoint = portals.get_portal_by_name(warpinfo.toname);
			pending_teleport = Teleport{ physics.get_y_below(spawnpoint), std::nullopt };

			player.show_effect_id(CharEffect::Id::TELEPORT);
			Sound(Sound::Name::PORTAL).play();
		} else if (warpinfo.valid)
		{
			// v83 asks to change map only when no request waits on the server
			// and 500 ms have passed since the last one, dead or alive
			// (FUN_0053035d).
			if (!player.request(ChangeMapPacket(field_key, warpinfo.name, player.get_position()), ExclRequest::WAIT, true))
				return;

			player.get_stats().set_mapid(warpinfo.mapid);

			Sound(Sound::Name::PORTAL).play();
		}
	}

	// v83 throws a player standing on a jump pad into the air and leaves a
	// burst where they took off (FUN_0094de22 for type 12, FUN_0094dac6 for
	// type 13).
	void Stage::use_jump_pad(const Portal &portal)
	{
		if (!player.get_phobj().onground || player.is_climbing())
			return;

		double hspeed;
		double vspeed;
		bool flip;

		if (portal.get_type() == Portal::SPRING1)
		{
			// 2300 px/s up and 200 px/s the way the player faces, with
			// 100 px/s more to the left or right for each arrow key held.
			bool left = !player.getflip();

			hspeed = left ? -200.0 : 200.0;

			if (player.is_key_down(KeyAction::Id::LEFT))
				hspeed -= 100.0;

			if (player.is_key_down(KeyAction::Id::RIGHT))
				hspeed += 100.0;

			vspeed = -2300.0;
			flip = !left;
		} else
		{
			// The portal's own speeds; the burst is mirrored whenever it
			// throws sideways, either way. v83 also requires a field type of
			// 19-21 or 25, or a reactor the portal names to be in state 1,
			// but every map with these pads passes and none names a reactor.
			Point<int16_t> impact = portal.get_impact();

			hspeed = impact.x();
			vspeed = -impact.y();
			flip = impact.x() != 0;
		}

		Physics::knockback(player.get_phobj(), hspeed, vspeed);
		ground_effects.add(jump_effect, DrawArgument(player.get_position(), flip));
	}

	// v83 makes a pending in-map move on the next update, by portal or by
	// skill, and allows no other for 600 ms after (FUN_0094e5e5).
	bool Stage::update_teleport()
	{
		if (teleport_cooldown > 0)
			teleport_cooldown -= Constants::TIMESTEP;

		if (!pending_teleport)
			return false;

		if (pending_teleport->foothold)
		{
			// A skill keeps the keys held, and puts the player on the floor
			// it chose, so they don't slide back along their last.
			player.set_position(pending_teleport->target);
			player.get_phobj().fhid = *pending_teleport->foothold;
		} else
		{
			player.respawn(pending_teleport->target, mapinfo.is_underwater());
		}

		pending_teleport.reset();
		teleport_cooldown = TELEPORT_COOLDOWN;

		return true;
	}

	// v83 checks a Teleport skill against the map and any pending move
	// before it looks for where to go (FUN_00957b74).
	void Stage::use_teleport(int32_t skill)
	{
		bool allowed = mapinfo.can_use_movement_skills() && !pending_teleport && teleport_cooldown <= 0;

		if (std::optional<teleport::Spot> spot = combat.use_teleport(skill, allowed))
		{
			pending_teleport = Teleport{ spot->position, spot->foothold };
			player.show_effect_id(CharEffect::Id::TELEPORT);

			// Element 4 marks the jump in the path, from where the player
			// stands until the move lands on the next update.
			player.record_move(4);
		}
	}

	void Stage::check_seats()
	{
		if (player.is_sitting() || player.is_attacking())
			return;

		Optional<const Seat> seat = mapinfo.findseat(player.get_position());
		player.set_seat(seat);
	}

	// After the player moves, v83 catches the ladder or rope the arrows ask
	// for, unless already climbing or attacking (FUN_009cbefb):
	// - In the air, Up catches one passed since the last update, but only
	//   while falling. The player keeps their height.
	// - On a foothold, Up catches one reaching within 20 px above the feet.
	//   Feet below its bottom rise to it.
	// - On a foothold, Down catches the first one reaching 10 px below the
	//   feet if its top is in that span, and puts the feet at its top.
	void Stage::catch_ladders(Point<int16_t> before)
	{
		if (player.is_climbing() || player.is_attacking())
			return;

		int8_t dir = player.is_key_down(KeyAction::Id::DOWN) - player.is_key_down(KeyAction::Id::UP);
		PhysicsObject &phobj = player.get_phobj();
		Point<int16_t> pos = player.get_position();

		if (!phobj.onground)
		{
			if (dir < 0 && phobj.vspeed > 0.0)
				if (Optional<const Ladder> ladder = mapinfo.findladder(before, pos))
					player.set_ladder(ladder);
		} else if (dir < 0)
		{
			if (Optional<const Ladder> ladder = mapinfo.findladder(pos - Point<int16_t>(0, 20), pos))
			{
				player.set_ladder(ladder);

				if (pos.y() > ladder->get_bottom())
					phobj.set_y(ladder->get_bottom());
			}
		} else if (dir > 0)
		{
			Optional<const Ladder> ladder = mapinfo.findladder(pos, pos + Point<int16_t>(0, 10));

			if (ladder && ladder->get_top() >= pos.y() && ladder->get_top() <= pos.y() + 10)
			{
				player.set_ladder(ladder);
				phobj.set_y(ladder->get_top());
			}
		}
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
							check_portals();
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
			// v83 uses skills and items only as their key goes down
			// (FUN_0094c856).
			case KeyType::Id::SKILL:
				if (down && teleport::teleports(action))
					use_teleport(action);
				else if (down)
					combat.use_move(action);

				break;
			case KeyType::Id::ITEM:
				if (down)
					player.use_item(action);

				break;
			case KeyType::Id::FACE:
				if (down && emote_cooldown.try_start(get_uptime() / 1000))
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

	int32_t Stage::other_character_at(Point<int16_t> cursorpos)
	{
		if (state != State::ACTIVE)
			return 0;

		Point<int16_t> viewpos = camera.position();

		// v83 hit-tests only the others (FUN_009726ea).
		for (auto &mmo : *chars.get_chars())
		{
			Optional<OtherChar> other = chars.get_char(mmo.first);

			if (other && other->is_active()
				&& char_click::menu_hits(other->get_position() + viewpos, other->is_prone(), other->getflip(), cursorpos))
				return other->get_oid();
		}

		return 0;
	}

	int32_t Stage::mini_room_at(Point<int16_t> cursorpos)
	{
		if (state != State::ACTIVE)
			return 0;

		Point<int16_t> viewpos = camera.position();

		for (auto &mmo : *chars.get_chars())
		{
			Optional<OtherChar> other = chars.get_char(mmo.first);

			if (!other || !other->is_active())
				continue;

			if (int32_t serial = other->mini_room_at(other->get_position() + viewpos, cursorpos))
				return serial;
		}

		return 0;
	}

	void Stage::release_key(KeyType::Id type, int32_t action)
	{
		if (state != State::ACTIVE)
			return;

		if (type == KeyType::Id::ACTION)
			player.release_key(KeyAction::actionbyid(action));

		// A skill held down goes off as its key comes up (0x95bedf).
		if (type == KeyType::Id::SKILL)
			combat.release_prepare(action);
	}

	void Stage::release_keys()
	{
		if (state != State::ACTIVE)
			return;

		player.release_keys();
		combat.finish_prepare();
	}

	void Stage::release_cursor()
	{
		player.release_chalkboard_cursor();
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

		// The close button on the player's own chalkboard, which needs both
		// the press and the release on it (FUN_0094fa8e). The board stays up
		// until the server takes it down. An open mini room blocks it.
		auto shop = UI::get().get_element<UIPersonalShop>();
		auto trade = UI::get().get_element<UITradingRoom>();
		bool mini_room = (shop && shop->is_active()) || (trade && trade->is_active());
		Point<int16_t> feet = player.get_position() + camera.position();

		if (!mini_room && player.send_chalkboard_cursor(feet, position, pressed, UI::get().is_cursor_pressed()))
		{
			CloseChalkboardPacket().dispatch();

			if (auto chatbar = UI::get().get_element<UIChatBar>())
				chatbar->send_chatline("The chalkboard has closed.", UIChatBar::LineType::SKILL);

			return Cursor::State::CANCLICK;
		}

		if (!mini_room && player.on_chalkboard_close(feet, position))
			return Cursor::State::CANCLICK;

		Cursor::State npcstate = npcs.send_cursor(pressed, position, camera.position());

		// A shop's balloon shows the house cursor (FUN_00531cc7).
		if (npcstate == Cursor::State::IDLE && mini_room_at(position))
			return Cursor::State::HOUSE;

		return npcstate;
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

	MapMists &Stage::get_mists()
	{
		return mists;
	}

	MapSummons &Stage::get_summons()
	{
		return summons;
	}

	MapDoors &Stage::get_doors()
	{
		return doors;
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

	const MapInfo &Stage::get_mapinfo() const
	{
		return mapinfo;
	}

	void Stage::set_field_key(uint8_t key)
	{
		field_key = key;
	}

	uint8_t Stage::get_field_key() const
	{
		return field_key;
	}

	void Stage::add_effect(std::string path)
	{
		effect = MapEffect(path);
	}

	void Stage::move_ship(uint8_t target, uint8_t event)
	{
		// One for the map being left is no longer news.
		if (state == State::ACTIVE)
			ship.move(target, event);
	}

	void Stage::show_ship_state(uint8_t state, uint8_t flag)
	{
		if (Stage::state == State::ACTIVE)
			ship.show_state(state, flag);
	}

	void Stage::set_clock(uint8_t hour, uint8_t minute, uint8_t second)
	{
		if (state != State::ACTIVE)
			pending_clock = ClockTime{ hour, minute, second };
		else if (clock.is_present())
			clock.set(hour, minute, second);
	}

	bool Stage::try_give_fame()
	{
		return fame_cooldown.try_give(get_uptime() / 1000);
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

	namespace
	{
		void notice(const char *message)
		{
			UI::get().emplace<UIOk>(message, [](bool) {});
		}

		// v83's notices 0x10a and 0x115.
		const char *const CATCH_BREATH = "You can't do it right now.\\nCatch your breath and then try again.";
		const char *const NOT_HERE = "You can't do it here in this map.";
	}

	// v83 FUN_005304af: the request gate, then the alert a hit or attack
	// leaves for 5 seconds (GMs aside), then fieldLimit.
	bool Stage::change_channel(uint8_t channel)
	{
		if (!player.can_request(ExclRequest::WAIT))
			return false;

		if (player.get_look().get_alerted() && !Configuration::get().get_admin())
		{
			notice(CATCH_BREATH);
			return false;
		}

		if (!mapinfo.can_migrate())
		{
			notice(NOT_HERE);
			return false;
		}

		return player.request(ChangeChannelPacket(channel), ExclRequest::WAIT);
	}

	// v83 FUN_00a04dca: the alert first, and the request gate without its
	// check that the player is alive.
	void Stage::enter_cash_shop()
	{
		if (player.get_look().get_alerted())
		{
			notice(CATCH_BREATH);
			return;
		}

		if (!player.can_request(ExclRequest::WAIT, true))
			return;

		if (!mapinfo.can_migrate())
		{
			notice(NOT_HERE);
			return;
		}

		player.request(EnterCashShopPacket(), ExclRequest::WAIT, true);
	}
}