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
#include "../Character/ItemEffectRules.h"
#include "../Character/Pets.h"
#include "QuestConversation.h"
#include "Timer.h"
#include "MapleMap/CharClick.h"
#include "Combat/FlashJump.h"
#include "Combat/Wings.h"
#include "FieldEffectRules.h"
#include "OxQuizRules.h"
#include "MapleMap/DoorRules.h"
#include "MapleMap/FieldLimit.h"
#include "MapleMap/FriendlyRules.h"
#include "MapleMap/KiteRules.h"
#include "MapleMap/Npc.h"

#include "../IO/UI.h"
#include "../IO/UiControl.h"
#include "../Util/Misc.h"
#include "../Util/NxFiles.h"
#include "../Util/Randomizer.h"

#include "../IO/UITypes/UIChatBar.h"
#include "../IO/UITypes/UIMobGage.h"
#include "../IO/UITypes/UINotice.h"
#include "../IO/UITypes/UINpcTalk.h"
#include "../IO/UITypes/UIQuiz.h"
#include "../IO/UITypes/UIStatusBar.h"
#include "../Net/Packets/AttackAndSkillPackets.h"
#include "../Net/Packets/GameplayPackets.h"
#include "../Net/Packets/MessagingPackets.h"
#include "../Data/SkillData.h"

#include <nlnx/nx.hpp>

#include <cmath>
#include <cstdlib>
#include <unordered_map>

namespace ms
{
	namespace
	{
		// An in-map portal's move lands 120 ms after Up; the portal then
		// waits 600 ms (v83 FUN_00957b74, FUN_0094e5e5).
		constexpr int16_t TELEPORT_COOLDOWN = 600;

		// How often v83 looks at the quest-bound tags again (FUN_00531b8d).
		constexpr int32_t QUEST_TAG_INTERVAL = 5000;

		// What a mob's attack shows on the player it hits: Mob.nx
		// <id>.img/attack<n>/info/hit, whether it stays on them (hit/attach),
		// and its CharDam sound.
		struct MobHit
		{
			Animation animation;
			bool shown;
			bool attach;
			Sound sound;
		};

		// The hit of the mob's attack `index` (attack1 is 0), read once
		// (FUN_0067fd81).
		const MobHit &mob_hit(int32_t mobid, int8_t index)
		{
			static std::unordered_map<int64_t, MobHit> loaded;

			int64_t key = static_cast<int64_t>(mobid) * 256 + index;
			auto iter = loaded.find(key);

			if (iter != loaded.end())
				return iter->second;

			std::string strid = string_format::extend_id(mobid, 7);
			nl::node hit = Mob::look_of(mobid)["attack" + std::to_string(index + 1)]["info"]["hit"];

			// Nothing shows for a hit without frames (info+0x78 empty).
			bool shown = false;

			for (nl::node frame : hit)
				shown = shown || frame.data_type() == nl::node::type::bitmap;

			// Its own sounds, or the linked mob's when it has none, as Mob's constructor takes them.
			nl::node sounds = nl::nx::sound["Mob.img"][strid];

			if (!sounds)
				if (nl::node link = nl::nx::mob[strid + ".img"]["info"]["link"])
					sounds = nl::nx::sound["Mob.img"][string_format::extend_id(link, 7)];

			Sound sound;

			if (const char *name = mob_attack::char_dam_sound(index))
				if (nl::node src = sounds[name])
					sound = Sound(src);

			return loaded.emplace(key, MobHit{ Animation(hit), shown, hit["attach"].get_bool(), sound }).first->second;
		}

		// The quest whose QuestInfo.img showLayerTag is `tag`, or 0. Every quest counts, not only the supported
		// ones (CQuestMan 0x722fd6).
		int16_t layer_tag_quest(const std::string &tag)
		{
			static const std::unordered_map<std::string, int16_t> quests = []()
			{
				std::unordered_map<std::string, int16_t> found;

				for (auto quest : nl::nx::quest["QuestInfo.img"])
				{
					std::string shown = quest["showLayerTag"].get_string();

					if (!shown.empty())
						found[shown] = string_conversion::or_default<int16_t>(quest.name(), 0);
				}

				return found;
			}();

			auto iter = quests.find(tag);

			return iter == quests.end() ? 0 : iter->second;
		}
	}

	Stage::Stage() : combat(player, chars, mobs, reactors, summons, drops, physics, mapinfo, portals)
	{
		state = State::INACTIVE;
	}

	void Stage::init()
	{
		drops.init();

		jump_effect = nl::nx::effect["BasicEff.img"]["VerticalJump"];
		fear_sight = nl::nx::effect["MapEff.img"]["Viewrange"]["3"];
		quest_alert.init();
	}

	void Stage::load(int32_t mapid, int8_t portalid, std::optional<Point<int16_t>> chase)
	{
		switch (state)
		{
			case State::INACTIVE:
				load_map(mapid);
				respawn(portalid, chase);
				guide.respawn(player.get_position());
				show_quest_tags();

				if (mapid != -1)
					quest_alert.enter_field(mapid);

				if (pending_clock && clock.is_present())
					clock.set(pending_clock->hour, pending_clock->minute, pending_clock->second);

				pending_clock.reset();

				if (pending_help)
					show_event_help(*pending_help);

				pending_help.reset();

				// The priest welcomes the couple as they come to the altar.
				for (const WeddingNews &news : pending_wedding)
					if (news.hearts)
						wedding.show_hearts();
					else
						wedding.progress(news.step, news.groom, news.bride, player.get_oid());

				pending_wedding.clear();

				if (pending_weather)
				{
					weather.set_packet_flag(pending_weather->flag);
					weather.blow(pending_weather->item_id, pending_weather->msg, pending_weather->from_map);
				}

				pending_weather.reset();

				// The player's effect item goes along, starting over where they arrive.
				player.restart_item_effect();

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
		// Another character starts without the last one's guide.
		if (entry.id != player.get_oid())
			guide.remove();

		player = entry;
		playable = player;
		ox_notice.clear();

		start = ContinuousTimer::get().start();
		emote_cooldown = {};
		fame_cooldown = {};
		item_effect_shown = -1;
		quest_alert.reset();

		CharStats stats = player.get_stats();
		levelBefore = stats.get_stat(MapleStat::Id::LEVEL);
		expBefore = stats.get_exp();
	}

	void Stage::clear()
	{
		state = State::INACTIVE;
		Sound::clear_listener();

		chars.clear();
		npcs.clear();
		mobs.clear();
		drops.clear();
		reactors.clear();
		mists.clear();
		kites.clear();
		merchants.clear();
		doors.clear();
		summons.clear();
		guide.hush();
		// The server brings the player's pets out again on the new field.
		player.clear_pets();
		player.glide(false, false);
		// And the aura of an item buff, as v83 makes the user anew (user+0x1f64).
		player.show_consume_effect(0);
		pending_clock.reset();
		pending_help.reset();
		pending_wedding.clear();
		pending_weather.reset();
		pending_weather_flag = 0;
	}

	void Stage::load_map(int32_t mapid)
	{
		Stage::mapid = mapid;

		// A copy of a map (a mini-dungeon's) keeps its own info but takes
		// everything else from the map it copies.
		nl::node src = mapid == -1 ? NxFiles::ui_classic["CashShopPreview.img"] : NxHelper::Map::get_map_body(mapid);
		nl::node info = mapid == -1 ? src["info"] : NxHelper::Map::get_map_node_name(mapid)["info"];

		physics = Physics(src["foothold"], info);
		Range<int16_t> spacex = physics.get_fht().get_space_x();
		Range<int16_t> spacey = physics.get_fht().get_space_y();
		tilesobjs = MapTilesObjs(src, Point<int16_t>(spacex.second() - spacex.first(), spacey.second() - spacey.first()));
		backgrounds = MapBackgrounds(src["back"]);
		mapinfo = MapInfo(info, src, physics.get_fht().get_walls(), physics.get_fht().get_borders());
		clouds = mapinfo.has_clouds() ? MapClouds(physics.get_fht().get_space_x(), physics.get_fht().get_space_y())
									  : MapClouds();
		portals = MapPortals(src["portal"], mapid);
		tooltips = mapid == -1 ? MapTooltips() : MapTooltips(mapid, src);
		UI::get().clear_tooltip(Tooltip::Parent::FIELD);
		ship = MapShip(src);
		snowfield = MapSnowball(src);
		wedding = mapid == -1 ? MapWedding() : MapWedding(mapid);
		clock = MapClock(src);
		ground_effects = EffectLayer();
		field_effects = EffectLayer();
		weather.stop_sounds();
		weather = mapid == -1 ? MapWeather() : MapWeather(src["weather"], mapinfo.get_bgm());
		jukebox = false;

		if (mapid != -1)
			player.get_stats().set_mapid(mapid);
	}

	void Stage::respawn(int8_t portalid, std::optional<Point<int16_t>> chase)
	{
		// v83's Cash Shop stage has no music of its own; the Cash Shop plays
		// its theme. A map's BGM fades the last one out, as PlayFieldBGM does
		// (FUN_0064211e).
		// A locked UI holds the map's back until it is unlocked (CField::Init).
		if (mapid == -1)
			Music("BgmUI.img/ShopBgm").play();
		else if (!ui_control::state().locked)
			Music(mapinfo.get_bgm()).play(field_effect::BGM_FADE);

		// A chased point is on the ground already, as a door is; look from
		// just above it, since its rounded y may sit below a slope.
		Point<int16_t> spawnpoint = chase ? *chase - Point<int16_t>(0, 5) : portals.get_portal_by_id(portalid);
		Point<int16_t> startpos = physics.get_y_below(spawnpoint);
		combat.clear_prepare();
		combat.clear_grenades();
		combat.clear_final_attack();
		player.set_flymap(mapinfo.is_flying());
		player.respawn(startpos, mapinfo.is_swimming(startpos));
		// v83 makes the user anew in each field, showing their orbs as they
		// are (FUN_0092e499).
		player.show_combo(player.get_buff_value(Buffstat::Id::COMBO) - 1, true);
		// And the aura of a Berserk it is in (0x9497c1).
		player.show_berserk(player.get_berserk() != 0);
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
			snowfield.draw(id, viewx, viewy, alpha);
			tilesobjs.draw(id, viewpos, alpha);
			doors.draw(id, town_doors, viewpos, alpha);
			reactors.draw(id, viewx, viewy, alpha);
			merchants.draw(id, viewx, viewy, alpha);
			npcs.draw(id, viewx, viewy, alpha);
			mobs.draw(id, viewx, viewy, alpha);
			chars.draw(id, viewx, viewy, alpha);

			// Bamboo Rain's black is just behind the player (z less 100).
			if (id == player.get_layer() && blackout_alpha > 0.0f)
				GraphicsGL::get().drawrectangle(0, 0, Constants::Constants::get().get_viewwidth(),
												Constants::Constants::get().get_viewheight(), 0.0f, 0.0f, 0.0f,
												blackout_alpha);

			player.draw(id, viewx, viewy, alpha);

			// The quest alert is in front of the player (z 3 at the face, FUN_00941417).
			if (id == player.get_layer())
				quest_alert.draw(player.get_phobj().get_absolute(viewx, viewy, alpha) + player.get_face_offset(), alpha);

			drops.draw(id, viewx, viewy, alpha);
			summons.draw(id, viewx, viewy, alpha);
		}

		summons.draw(-1, viewx, viewy, alpha);
		guide.draw(viewx, viewy, alpha);
		clock.draw(viewx, viewy);
		ship.draw_above(viewx, viewy, alpha);

		const Party &party = player.get_party();

		if (party.shows_hp_marks())
			chars.draw_party_hp(party, player.get_oid(), mapid, viewx, viewy, alpha);

		// Kites share the mini room balloons' z (0xc0061440), in front of
		// everyone and behind the effect layer.
		kites.draw(viewx, viewy, alpha);
		merchants.draw_balloons(viewx, viewy);
		// v83 puts a mist's clouds on the effect layer, above the map's
		// objects (z 0xc00614a4).
		mists.draw(viewx, viewy, alpha);
		// So are a wedding's hearts.
		wedding.draw(Point<int16_t>(static_cast<int16_t>(std::round(viewx)), static_cast<int16_t>(std::round(viewy))),
					 alpha);
		// A weather's bursts and fireworks share the mists' layer (z 0xc00614a4).
		weather.draw_shows(alpha);
		drops.draw_explosions(viewpos, alpha);
		combat.draw(viewx, viewy, alpha);
		snowfield.draw_front(viewx, viewy, alpha);
		ground_effects.drawabove(viewpos, alpha);
		portals.draw(viewpos, alpha);
		backgrounds.drawforegrounds(viewx, viewy, alpha);
		// v83's clouds are above the front backgrounds and below the fear
		// overlay (z 0xc0061508).
		clouds.draw(viewx, viewy, alpha);
		// A weather's particles are on the clouds' layer.
		weather.draw(viewx, viewy, alpha);
		draw_fear(viewx, viewy, alpha);
		// The tips of the zones the player stands in are at z 0, above the
		// field's own layers (FUN_008f3141).
		tooltips.draw(viewpos);
		field_effects.drawabove(viewpos, alpha);
		// The weather's message bar is above the field effects (z 0xc00616fc).
		weather.draw_bar();
		// OX_QUIZ's notice is one above it (z 0xc00616fd).
		ox_notice.draw();
		screen_effects.drawabove(Point<int16_t>(0, 0), alpha);
	}

	void Stage::draw_overlay(float alpha) const
	{
		if (state != State::ACTIVE)
			return;

		overlay_effects.drawabove(Point<int16_t>(0, 0), alpha);
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

		Sound::set_listener(player.get_position());

		if (auto step = macro_runner.due(Constants::TIMESTEP))
			macro_runner.tried(try_macro_step(*step));

		backgrounds.update();
		clouds.update();
		weather.update();
		ship.update();
		wedding.update(position_of(wedding.get_groom()), position_of(wedding.get_bride()));
		clock.update();
		ground_effects.update();
		field_effects.update();
		screen_effects.update();
		overlay_effects.update();

		// Coconut Season's nuts take their states as they come due (FUN_005496c7).
		if (coconut::on_field())
			for (const coconut::Hit &hit : coconut::state().hits.take_due(coconut::now()))
				tilesobjs.set_state(coconut::object_name(hit.index), hit.state, player.get_position());

		// Not on the last map while it fades out.
		if (snowfield.is_active())
			touch_snowball();

		snowfield.update();
		tilesobjs.update();

		// Every 5 s, and whenever a quest record changes (FUN_00a20f4c).
		since_quest_tags += Constants::TIMESTEP;

		if (since_quest_tags > QUEST_TAG_INTERVAL || player.get_quests().revision() != quest_tags_revision)
			show_quest_tags();

		quest_alert.update();

		reactors.update(physics);
		mists.update();
		kites.update();
		merchants.update(physics);
		doors.update(physics);
		npcs.update(physics);
		Point<int16_t> playerpos = player.get_position();
		mob_attack::Box playerbody = mob_attack::user_body(playerpos.x(), playerpos.y(), player.is_prone(),
														   !player.getflip());

		// Hits on a friendly mob are reported by the party's leader, or by
		// a player alone on the map (0x667ed5).
		bool reporter = friendly::reports(player.get_party().is_leader(player.get_oid()), chars.size());

		mobs.update(physics, { player.get_oid(), playerpos, playerbody, player.get_phobj().onground,
							   player.is_dead(), player.has_buff(Buffstat::Id::DARKSIGHT), player.is_barrelled(),
							   player.get_beacon() }, reporter);
		chars.update(physics);
		drops.update(physics);

		// v83 tests touch portals and jump pads against where the player is
		// drawn, between its last two 30 ms physics steps (FUN_009b6205), so
		// a player landing is still tested above the floor. The nearest here
		// is where this step started, unless the player was moved there.
		Point<int16_t> drawn = player.get_position();

		Point<int16_t> before = drawn;
		sweep_pets();
		// A swimArea's water is where the player's feet are (IsSwimming
		// FUN_007046ac).
		player.set_underwater(mapinfo.is_swimming(drawn));
		player.update(physics);
		catch_ladders(before);

		if (update_teleport())
			drawn = player.get_position();

		// The player's summons follow where the player now is.
		summons.update(physics, player.get_position(), player.is_climbing());
		guide.update(physics, player.get_position(), player.is_climbing());
		check_pet_loot();
		show_rings();

		portals.update(player.get_position());
		check_touch_portals(drawn);

		auto now = static_cast<int32_t>(get_uptime() / 1000);

		if (now >= next_shake)
		{
			next_shake = now + tremble::TICK;
			auto offset = tremble::step(shake, now, std::rand(), std::rand());
			camera.shake({ static_cast<int16_t>(offset.x), static_cast<int16_t>(offset.y) });
		}

		blackout_alpha = blackout.update(now);

		camera.update(player.get_position());
		tooltips.update(player.get_position());

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
			player.check_auto_hp(result.hp_loss, result.guarded);
			combat.interrupt_prepare(result.damage);

			if (result.damage > 0)
				show_mob_hit(player, result.mobid, result.index, result.direction != 0);

			if (result.dodged)
				combat.show_player_special(result.dodged);

			if (result.stance)
				combat.show_stance(result.stance_skill);

			// What Mana Reflection sent back shows over the mob.
			if (result.reflect)
				mobs.show_damage(result.oid, result.reflected);
		}

		if (player.is_dead() || player.Char::is_invincible() || sheltered)
			return;

		int32_t touching = mobs.find_colliding(player.get_phobj(), mapinfo.get_fieldlimit(),
				player.has_buff(Buffstat::Id::DARKSIGHT));

		if (touching)
			combat.body_attack(touching);

		if (touching && player.is_barrelled())
		{
			// A mob touching the barrel does no harm, but every 2 seconds may see through it
			// (0x94b3ef, 0x94b65d).
			int64_t now = get_uptime() / 1000;

			if (now - barrel_checked >= 2000)
			{
				barrel_checked = now;
				int32_t level = player.get_skills().get_level(morph::OAK_BARREL);
				float chance = SkillData::get(morph::OAK_BARREL).get_stats(level).chance;

				if (morph::spotted(static_cast<int32_t>(std::lround(chance * 100)), Randomizer().next_int<uint32_t>(100)))
					CancelBuffPacket(morph::OAK_BARREL).dispatch();
			}
		} else if (touching)
		{
			if (MobAttack attack = mobs.create_attack(touching))
			{
				MobAttackResult result = player.damage(attack);
				TakeDamagePacket(result).dispatch();
				player.check_auto_hp(result.hp_loss, result.guarded);
				combat.interrupt_prepare(result.damage);

				if (result.dodged)
					combat.show_player_special(result.dodged);

				if (result.stance)
					combat.show_stance(result.stance_skill);

				// What Power Guard sent back shows over the mob.
				if (result.reflect)
					mobs.show_damage(result.oid, result.reflected);
			}
		}

		// v83 tests the traps whether or not a mob hit; a hit leaves the player invincible for the trap (0x94b5e8).
		hit_obstacle();

		if (!touching && since_mist_hurt > mist::HURT_INTERVAL)
		{
			// Not touching a mob, the player in a mob's mist hurts
			// themselves its x, without a knockback or invincibility
			// (FUN_00431783 at 0x94b7ba).
			mist::Hurt hurt;

			// Less the job's resistance to its element (0x94b7f2).
			if (mists.hurt_at(player.get_position(), hurt) &&
				(hurt.damage = mist::resisted(hurt.damage, player.resists(mist::resisted_as(hurt.code >> 8)))) > 0)
			{
				player.show_damage(hurt.damage);
				TakeDamagePacket(hurt.element, hurt.damage, hurt.code).dispatch();
				player.check_auto_hp(hurt.damage);
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
			player.check_auto_hp(damage);
		}
	}

	void Stage::hit_obstacle()
	{
		// The body the mob touch takes (I: FUN_0045183b for both).
		const PhysicsObject &phobj = player.get_phobj();
		Range<int16_t> horizontal(phobj.get_last_x(), phobj.get_x());
		Range<int16_t> vertical(phobj.get_last_y(), phobj.get_y());
		obstacle::Box body{ horizontal.smaller(), vertical.smaller() - 50, horizontal.greater(), vertical.greater() };

		const Obj *trap = tilesobjs.find_obstacle(body);

		if (!trap)
			return;

		const obstacle::Trap &data = trap->get_trap();
		int32_t damage = obstacle::roll(data.damage, static_cast<uint32_t>(std::rand()));
		uint16_t code = obstacle::code(damage, data);
		damage = obstacle::forced(damage, player.get_stats().get_stat(MapleStat::Id::HP), data.force_hp);

		if (damage > 0 && !player.Char::is_invincible())
		{
			obstacle::Vec push = trap->get_push();

			if (push == obstacle::Vec{ 0, 0 })
				push = obstacle::fallback(phobj.get_last_x(), phobj.get_last_y(), phobj.get_x(), phobj.get_y());

			auto [hp_loss, guarded] = player.damage_by_obstacle(damage, push.x, push.y);
			TakeDamagePacket(TakeDamagePacket::Obstacle{ damage, code }).dispatch();
			player.check_auto_hp(hp_loss, guarded);
		}

		// Witch Tower's snacks send the player back to its entrance, hurting or not (0x94b767).
		if (data.target_field != 0)
			player.request(ChangeMapPacket(field_key, data.target_field, "", false), ExclRequest::WAIT, true);
	}

	void Stage::touch_snowball()
	{
		const snowball::Rules &rules = snowball::rules();
		const snowball::State &game = snowball::state();
		Point<int16_t> position = player.get_position();

		if (snowball_knock_wait > 0)
			snowball_knock_wait -= Constants::TIMESTEP;

		// In the first ball the player stands in, unless it is fading out
		// or they are stunned (FUN_0057502e).
		for (int32_t lane = 0; lane < 2; lane++)
		{
			if (!rules.ball(lane, game.balls[lane].step).contains(position.x(), position.y()))
				continue;

			if (game.state != lane + 2 && !player.has_buff(Buffstat::Id::STUN) && snowball_knock_wait <= 0)
			{
				LeftKnockbackPacket().dispatch();
				snowball_knock_wait = snowball::TICK;
			}

			break;
		}

		// A snowman hurts as a trap would, and pushes the player right and
		// up (user vtable +0x48 with code 0).
		for (size_t lane = 0; lane < 2; lane++)
		{
			if (!rules.snowman(lane).contains(position.x(), position.y()))
				continue;

			if (!player.is_dead() && !player.Char::is_invincible())
			{
				int32_t damage = snowball::touch_damage(std::rand());
				auto [hp_loss, guarded] = player.damage_by_obstacle(damage, 270, -270);
				TakeDamagePacket(TakeDamagePacket::Obstacle{ damage, 0 }).dispatch();
				player.check_auto_hp(hp_loss, guarded);
			}

			break;
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

	void Stage::teleport_to_portal(uint8_t id)
	{
		if (!portals.has_portal(id))
			return;

		pending_teleport = Teleport{ physics.get_y_below(portals.get_portal_by_id(id)), std::nullopt };

		player.show_effect_id(CharEffect::Id::TELEPORT);
		Sound(Sound::Name::PORTAL).play();
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
			player.respawn(pending_teleport->target, mapinfo.is_swimming(pending_teleport->target));
		}

		pending_teleport.reset();
		teleport_cooldown = TELEPORT_COOLDOWN;

		return true;
	}

	bool Stage::may_dash() const
	{
		return !pending_teleport;
	}

	bool Stage::is_active() const
	{
		return state == State::ACTIVE;
	}

	void Stage::dash(teleport::Spot spot)
	{
		// Element 8 marks the dash in the path, from where the player
		// stands until it lands on the next update.
		player.record_move(8);
		pending_teleport = Teleport{ spot.position, spot.foothold };
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

	// v83 lets Flash Jump go only in the air, and leaves its effect where
	// the player was (FUN_0096be38).
	void Stage::use_flash_jump(int32_t skill)
	{
		if (!combat.use_flash_jump(skill, mapinfo.can_use_movement_skills(), player.may_flash_jump()))
			return;

		player.flash_jump(player.get_skills().get_level(skill));
		show_flash_jump(flash_jump::effect(skill), player.get_position(), player.getflip());
	}

	// v83 casts Wings from a foothold or the water, jumps, and glides the
	// player down once falling (FUN_00957a1f). The jump can still fail
	// after the cast, as where the map forbids jumping (FUN_009b21da).
	void Stage::use_wings(int32_t skill)
	{
		if (!combat.use_wings(skill, mapinfo.can_use_movement_skills(), player.may_wings()))
			return;

		if (!field_limit::may_jump(mapinfo.get_fieldlimit(), false))
			return;

		const SkillData::Stats &stats = SkillData::get(skill).get_stats(player.get_skills().get_level(skill));
		player.wings(stats.x, stats.time);
	}

	void Stage::show_flash_jump(const char *effect, Point<int16_t> position, bool right)
	{
		ground_effects.add(nl::nx::effect["BasicEff.img"][effect], DrawArgument(position, right));
	}

	nl::node Stage::effect_node(const std::string &path)
	{
		size_t slash = path.find('/');

		if (slash == std::string::npos)
			return nl::node();

		std::string file = path.substr(0, slash);
		nl::node root = file == "UI" ? nl::nx::ui : file == "Effect" ? nl::nx::effect : file == "Map" ? nl::nx::map
			: file == "Sound" ? nl::nx::sound : nl::node();

		if (!root)
			return root;

		std::string node = path.substr(slash + 1);

		if (nl::node found = root.resolve(node))
			return found;

		std::string image = field_effect::with_image(node);

		return image.empty() ? nl::node() : root.resolve(image);
	}

	void Stage::show_ground_effect(nl::node effect, Point<int16_t> position)
	{
		ground_effects.add(effect, DrawArgument(position));
	}

	void Stage::show_reward_effect(Point<int16_t> position, int32_t item_id, const std::string &path)
	{
		Sound(item_id).play();

		if (nl::node effect = effect_node(path))
			ground_effects.add(effect, DrawArgument(position));
	}

	void Stage::tremble(double force, bool heavy, int32_t delay, int32_t additional, bool forced)
	{
		if (!forced && !Setting<ShakeScreen>::get().load())
			return;

		shake = tremble::set(force, heavy, delay, additional, static_cast<int32_t>(get_uptime() / 1000));
	}

	void Stage::black_out(int32_t layers)
	{
		blackout.add(static_cast<int32_t>(get_uptime() / 1000), layers);
	}

	void Stage::check_seats()
	{
		// Not on a mount, morphed or a ghost, wherever the player is (FUN_0094e45f).
		if (player.is_sitting() || player.is_attacking() || player.ride_refuses() || player.morph_refuses() ||
			player.ghost_refuses())
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

	void Stage::show_rings()
	{
		for (RingKind kind : { RingKind::CRUSH, RingKind::FRIENDSHIP, RingKind::WEDDING })
		{
			std::vector<RingWearer> wearers = chars.ring_wearers(kind);

			if (auto own = player.ring_wearer(kind))
				wearers.push_back(*own);

			auto effects = marriage::ring_effects(wearers, kind);
			RingEffect own;

			for (auto &[id, effect] : effects)
				if (id == player.get_oid())
					own = effect;

			chars.show_rings(kind, effects);
			player.show_ring(kind, own);
		}
	}

	std::vector<int32_t> Stage::pets_ignore()
	{
		std::vector<int32_t> ignored;

		if (player.get_pet(0) && player.pet_wears(0, pets::ITEM_IGNORE))
			for (uint8_t index = 0; index < 3; index++)
			{
				Optional<PetLook> pet = player.get_pet(index);

				if (!pet)
					break;

				if (player.pet_wears(index, pets::ITEM_IGNORE))
					ignored.insert(ignored.end(), pet->get_ignored().begin(), pet->get_ignored().end());
			}

		return ignored;
	}

	void Stage::sweep_pets()
	{
		auto wears = [&](uint8_t index, int32_t itemid) {
			return player.pet_wears(index, itemid);
		};

		// Not for a dead owner, or in the Extra Frosty Snow Zone, as with
		// looting.
		bool sweeping = !player.is_dead() && mapid != 209080000;
		std::vector<int32_t> ignored = sweeping ? pets_ignore() : std::vector<int32_t>();
		const std::vector<int32_t> none;
		std::array<std::optional<MapDrops::Sweeper>, 3> sweepers;

		for (uint8_t index = 0; sweeping && index < 3; index++)
		{
			Optional<PetLook> pet = player.get_pet(index);

			// A pet uses the ignore list only if it wears a pendant itself.
			if (pet && wears(index, pets::SWEEPER))
				sweepers[index] = MapDrops::Sweeper{ pet->get_position(), wears(index, pets::ITEM_POUCH),
													 wears(index, pets::MAGIC_SCALES), wears(index, pets::LONG_RANGE),
													 wears(index, pets::ITEM_IGNORE) ? &ignored : &none };
		}

		std::array<int8_t, 3> directions =
			drops.sweep(player.get_position(), sweepers, player.get_oid(), player.get_party().get_id());

		for (uint8_t index = 0; index < 3; index++)
		{
			Optional<PetLook> pet = player.get_pet(index);

			if (!pet)
				continue;

			// A pet takes a drop even if it can't loot, but only one that
			// loots walks to it (FUN_009c4122).
			bool loots = wears(index, pets::MESO_MAGNET) || wears(index, pets::ITEM_POUCH);
			pet->set_sweep(loots ? directions[index] : 0);
			pet->set_long_range(wears(index, pets::LONG_RANGE));
		}
	}

	void Stage::check_pet_loot()
	{
		// Pets don't loot for a dead owner, or in the Extra Frosty Snow Zone
		// (FUN_0070403d).
		if (player.is_dead() || mapid == 209080000)
			return;

		auto wears = [&](uint8_t index, int32_t itemid) {
			return player.pet_wears(index, itemid);
		};

		std::vector<int32_t> ignored = pets_ignore();

		for (uint8_t index = 0; index < 3; index++)
		{
			Optional<PetLook> pet = player.get_pet(index);

			if (!pet || !pet->is_on_ground())
				continue;

			bool mesos = wears(index, pets::MESO_MAGNET);
			bool items = wears(index, pets::ITEM_POUCH);

			if (!mesos && !items)
				continue;

			bool others = wears(index, pets::MAGIC_SCALES);
			Point<int16_t> position = pet->get_position();

			if (int32_t oid = drops.find_pet_loot_at(position, player.get_oid(), player.get_party().get_id(), mesos,
													 items, others, ignored))
				PetLootPacket(pet->get_serial(), position, oid, others, wears(index, pets::SWEEPER),
							  wears(index, pets::LONG_RANGE))
					.dispatch();
		}
	}

	void Stage::send_key(KeyType::Id type, int32_t action, bool down)
	{
		if (state != State::ACTIVE || !playable || player.is_dead())
			return;

		// While one of v83's dialogs is open the field takes no key going
		// down (FUN_00529968): no skill, item, face, macro, attack, jump,
		// pickup, portal or arrow. Releases still let go of held keys.
		if (down && UI::get().dialog_open())
			return;

		// Any other bound key going down drops the macro running (v83
		// FUN_0094f2ed), and so do Left and Right going down or up
		// (FUN_0094c856). Down isn't bound in v83.
		bool walking = type == KeyType::Id::ACTION
					   && (action == KeyAction::Id::LEFT || action == KeyAction::Id::RIGHT);
		bool crouching = type == KeyType::Id::ACTION && action == KeyAction::Id::DOWN;

		if (walking || (down && type != KeyType::Id::MACRO && !crouching))
			macro_runner.clear();

		// A double tap on Left or Right dashes, and letting either go ends the Dash (FUN_0074d993, 0x9678e1).
		int32_t arrow = !walking ? 0 : action == KeyAction::Id::LEFT ? 1 : 2;
		bool dashing = player.has_buff(Buffstat::Id::DASH_SPEED);
		int32_t dash_skill = dash::skill(player.get_stats().get_job().get_id());

		bool tapped = dash_command.feed(arrow, down, get_uptime() / 1000, dashing);

		if (tapped && player.get_skills().get_level(dash_skill) > 0)
			combat.use_move(dash_skill);
		else if (walking && !down && dashing)
			CancelBuffPacket(dash_skill).dispatch();

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
				else if (down && flash_jump::jumps(action))
					use_flash_jump(action);
				else if (down && wings::is_wings(action))
					use_wings(action);
				else if (down)
					combat.use_move(action);

				break;
			case KeyType::Id::ITEM:
				if (down)
					player.use_item(action);

				break;
			// An item effect, while the player has the item (FUN_00a246e0).
			case KeyType::Id::EFFECT:
				if (down && player.get_inventory().get_total_item_count(action) > 0)
					use_item_effect(action);

				break;
			// The macro at the key's place in the list, unless a skill is held
			// down (FUN_0094f2ed).
			case KeyType::Id::MACRO:
				if (down && !combat.is_preparing())
					macro_runner.start(UI::get().get_skill_macros(), action, [](int32_t skill)
					{
						return SkillData::get(skill).has_cooltime();
					});

				break;
			case KeyType::Id::FACE:
				// Not morphed (FUN_00a24470).
				if (down && !player.shape_refuses() && emote_cooldown.try_start(get_uptime() / 1000))
				{
					player.set_expression(action);
					// v83 numbers emotes from F1's action (100) as 1.
					FaceExpressionPacket(action - 99).dispatch();
				}

				break;
		}
	}

	void Stage::stop_macro()
	{
		macro_runner.clear();
	}

	bool Stage::try_macro_step(const skill_macro::Step& step)
	{
		// In Dark Sight only Assassinate goes. (v83 lets Vanish's owner use
		// any skill; there is no Vanish here.)
		constexpr int32_t ASSASSINATE = 4221001;

		if (player.has_buff(Buffstat::Id::DARKSIGHT) && step.skill != ASSASSINATE)
			return false;

		// Nor does anything while a dialog is open (FUN_0063196b).
		if (UI::get().dialog_open())
			return false;

		if (!combat.use_move(step.skill))
			return false;

		const SkillMacros &macros = UI::get().get_skill_macros();

		// The first skill to go off says the macro's name, in a balloon only.
		if (step.first && macros.shouts(step.macro) && UI::get().get_element<UIChatBar>())
			GeneralChatPacket(macros.name(step.macro), true).dispatch();

		return true;
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

	const MapKites::Kite *Stage::kite_at(Point<int16_t> cursorpos) const
	{
		if (state != State::ACTIVE)
			return nullptr;

		return kites.at(cursorpos - camera.position());
	}

	std::optional<MapMerchants::Hit> Stage::merchant_at(Point<int16_t> cursorpos) const
	{
		if (state != State::ACTIVE)
			return std::nullopt;

		return merchants.at(cursorpos - camera.position());
	}

	std::optional<MiniRoomHit> Stage::mini_room_at(Point<int16_t> cursorpos)
	{
		if (state != State::ACTIVE)
			return std::nullopt;

		Point<int16_t> viewpos = camera.position();

		for (auto &mmo : *chars.get_chars())
		{
			Optional<OtherChar> other = chars.get_char(mmo.first);

			if (!other || !other->is_active())
				continue;

			if (std::optional<MiniRoomHit> hit = other->mini_room_at(other->get_position() + viewpos, cursorpos))
				return hit;
		}

		return std::nullopt;
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
		quest_alert.release_cursor();
	}

	Cursor::State Stage::send_cursor(bool pressed, Point<int16_t> position)
	{
		// The field shows a boss's name over the icon on its HP bar (0x529a09).
		if (auto gage = UI::get().get_element<UIMobGage>())
			gage->hover(position);

		auto statusbar = UI::get().get_element<UIStatusBar>();

		if (statusbar && statusbar->is_menu_active())
		{
			if (pressed)
				statusbar->remove_menus();

			if (statusbar->is_in_range(position))
			{
				tooltips.leave();

				return statusbar->send_cursor(pressed, position);
			}
		}

		// The close button on the player's own chalkboard, which needs both
		// the press and the release on it (FUN_0094fa8e). The board stays up
		// until the server takes it down. While a dialog is open the field
		// takes no click (FUN_005299de), and the board no hover either.
		bool locked = UI::get().dialog_open();
		Point<int16_t> feet = player.get_position() + camera.position();

		if (!locked && player.send_chalkboard_cursor(feet, position, pressed, UI::get().is_cursor_pressed()))
		{
			CloseChalkboardPacket().dispatch();

			if (auto chatbar = UI::get().get_element<UIChatBar>())
				chatbar->send_chatline("The chalkboard has closed.", UIChatBar::LineType::SKILL);

			tooltips.leave();

			return Cursor::State::CANCLICK;
		}

		if (!locked && player.on_chalkboard_close(feet, position))
		{
			tooltips.leave();

			return Cursor::State::CANCLICK;
		}

		// Nor do the NPCs or the quest alert, so no hand shows over them.
		Cursor::State npcstate = locked ? Cursor::State::IDLE : npcs.send_cursor(pressed, position, camera.position());

		// Then the quest alert (FUN_0094fa8e).
		if (!locked && npcstate == Cursor::State::IDLE)
			quest_alert.send_cursor(player.get_position() + player.get_face_offset() + camera.position(), position,
									pressed, UI::get().is_cursor_pressed());

		// A shop's balloon shows the house cursor, a game's the game one
		// (FUN_00531cc7).
		if (npcstate == Cursor::State::IDLE)
			if (std::optional<MiniRoomHit> hit = mini_room_at(position))
				npcstate = hit->game ? Cursor::State::GAME : Cursor::State::HOUSE;

		// Then a kite shows whose it is, with the arrow (FUN_0094cfc3).
		const MapKites::Kite *flown = npcstate == Cursor::State::IDLE ? kite_at(position) : nullptr;

		if (flown)
		{
			tooltips.leave();
			UI::get().show_line(Tooltip::Parent::FIELD, kite::hover_text(flown->name, flown->item_msg));
		}
		// Otherwise a map zone under the cursor shows its tip.
		else if (npcstate == Cursor::State::IDLE)
			tooltips.hover(position - camera.position());
		else
			tooltips.leave();

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

	MapKites &Stage::get_kites()
	{
		return kites;
	}

	MapMists &Stage::get_mists()
	{
		return mists;
	}

	MapSummons &Stage::get_summons()
	{
		return summons;
	}

	Guide &Stage::get_guide()
	{
		return guide;
	}

	MapMerchants &Stage::get_merchants()
	{
		return merchants;
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

	QuestAlert &Stage::get_quest_alert()
	{
		return quest_alert;
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

	bool Stage::add_effect(std::string path)
	{
		nl::node src = nl::nx::map["Effect.img"].resolve(path);

		if (!src)
			return false;

		// One-time and stacking, at z 0xc00614a4 below the UI unless the node has its own z, as
		// only the miro nodes do (FUN_005330f7).
		auto position = field_effect::screen_position(Constants::Constants::get().get_viewwidth(), Constants::Constants::get().get_viewheight());
		EffectLayer &layer = src["z"] ? overlay_effects : screen_effects;
		layer.add(src, DrawArgument(Point<int16_t>(position.x, position.y)));

		return true;
	}

	void Stage::add_summon_effect(uint8_t type, Point<int16_t> position)
	{
		std::string id = std::to_string(type);
		nl::node effect = nl::nx::effect["Summon.img"][id];

		if (effect)
			field_effects.add(effect, DrawArgument(position));

		nl::node sound = nl::nx::sound["Summon.img"][id];

		if (sound)
		{
			Point<int16_t> distance = position - player.get_position();
			double length = std::sqrt(static_cast<double>(distance.x()) * distance.x() + static_cast<double>(distance.y()) * distance.y());

			Sound(sound).play(field_effect::summon_volume(length));
		}
	}

	void Stage::show_mob_hit(Char &target, int32_t mobid, int8_t index, bool left)
	{
		if (index < 0)
			return;

		const MobHit &hit = mob_hit(mobid, index);

		if (!hit.shown)
			return;

		Point<int16_t> position = target.get_position();
		mob_attack::Box body = mob_attack::user_body(position.x(), position.y(), target.is_prone(), !target.getflip());
		mob_attack::Spot spot = mob_attack::hit_spot(body, static_cast<uint32_t>(std::rand()), static_cast<uint32_t>(std::rand()));
		Point<int16_t> at(static_cast<int16_t>(spot.x), static_cast<int16_t>(spot.y));

		if (hit.attach)
			target.show_attack_effect(hit.animation, 1);
		else
			field_effects.add(hit.animation, DrawArgument(at, left));

		Point<int16_t> distance = at - player.get_position();
		double length = std::sqrt(static_cast<double>(distance.x()) * distance.x() + static_cast<double>(distance.y()) * distance.y());
		Sound sound = hit.sound;
		sound.play(field_effect::summon_volume(length));
	}

	void Stage::set_object_state(const std::string &name, int32_t state)
	{
		tilesobjs.set_state(name, state, player.get_position());
	}

	std::vector<coconut::Box> Stage::coconut_boxes() const
	{
		std::unordered_map<std::string, Rectangle<int16_t>> named = tilesobjs.named_boxes();
		std::vector<coconut::Box> boxes;

		for (int16_t i = 0; i < coconut::count(); i++)
		{
			auto iter = named.find(coconut::object_name(i));

			if (iter == named.end())
				boxes.push_back({ 0, 0, 0, 0 });
			else
				boxes.push_back({ iter->second.left(), iter->second.top(), iter->second.right(), iter->second.bottom() });
		}

		return boxes;
	}

	void Stage::reset_obstacles()
	{
		tilesobjs.reset_obstacles(player.get_position());
	}

	void Stage::show_tag(const std::string &tag, bool shown)
	{
		tilesobjs.show_tags([&](const std::string &other) { return other == tag ? std::optional<bool>(shown) : std::nullopt; });
	}

	void Stage::fade_back(uint8_t mode, uint8_t index, int32_t duration)
	{
		if (std::optional<float> target = field_effect::back_target(mode))
			backgrounds.fade(index, *target, duration);
	}

	void Stage::reset_backs()
	{
		backgrounds.reset();
	}

	void Stage::show_quest_tags()
	{
		QuestLog &quests = player.get_quests();

		tilesobjs.show_tags([&](const std::string &tag) -> std::optional<bool>
		{
			if (int16_t quest = layer_tag_quest(tag))
				return quests.is_started(quest);

			return std::nullopt;
		});

		since_quest_tags = 0;
		quest_tags_revision = quests.revision();
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

	std::optional<Point<int16_t>> Stage::position_of(int32_t cid)
	{
		if (cid == player.get_oid())
			return player.get_position();

		if (auto other = chars.get_char(cid))
			return other->get_position();

		return std::nullopt;
	}

	void Stage::wedding_progress(uint8_t step, int32_t groom, int32_t bride)
	{
		if (state == State::ACTIVE)
			wedding.progress(step, groom, bride, player.get_oid());
		else
			pending_wedding.push_back({ false, step, groom, bride });
	}

	void Stage::wedding_hearts()
	{
		if (state == State::ACTIVE)
			wedding.show_hearts();
		else
			pending_wedding.push_back({ true, 0, 0, 0 });
	}

	void Stage::answer_priest(uint32_t page, int8_t response)
	{
		wedding.answered(page, response);
	}

	void Stage::set_notice(const std::string &msg)
	{
		if (msg.empty())
			ox_notice.clear();
		else
			ox_notice.show(msg, ox_quiz::FLOAT_TYPE);
	}

	void Stage::show_event_help(uint8_t index)
	{
		if (state != State::ACTIVE)
		{
			pending_help = index;
			return;
		}

		nl::node strings = nl::nx::string["Map.img"][NxHelper::Map::get_map_category(mapid)][std::to_string(mapid)];
		std::string text = gm_event::help_line(index, [&strings](uint8_t i)
		{
			return strings["help" + std::to_string(i)].get_string();
		});

		if (text.empty())
			return;

		// Not over an NPC's conversation with the server.
		if (auto talk = UI::get().get_element<UINpcTalk>(); talk && talk->is_active() && !talk->is_local())
			return;

		UI::get().emplace<UINpcTalk>();
		UI::get().enable();

		if (auto talk = UI::get().get_element<UINpcTalk>())
		{
			talk->change_text(gm_event::NPC, UINpcTalk::SENDOK, 0, 0, text);
			// Only the dialog closes; nothing goes back to the server.
			talk->set_local_responder([](int8_t, int32_t) {});
		}
	}

	void Stage::blow_weather(int32_t item_id, const std::string &msg, bool from_map)
	{
		if (state != State::ACTIVE)
			pending_weather = WeatherNews{ item_id, msg, from_map, pending_weather_flag };
		else
			weather.blow(item_id, msg, from_map);
	}

	void Stage::set_weather_flag(uint8_t flag)
	{
		if (state != State::ACTIVE)
			pending_weather_flag = flag;
		else
			weather.set_packet_flag(flag);
	}

	void Stage::start_jukebox()
	{
		jukebox = true;
	}

	void Stage::restart_bgm()
	{
		if (state != State::ACTIVE || mapid == -1)
			return;

		jukebox = false;
		weather.restart_bgm();
	}

	bool Stage::effect_busy() const
	{
		return jukebox || weather.busy();
	}

	void Stage::note_mob_left()
	{
		weather.mob_left();
	}

	void Stage::set_field_bgm(const std::string &path)
	{
		weather.set_bgm_override(path);
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

	void Stage::use_item_emotion(int32_t item_id)
	{
		nl::node faces = nl::nx::item["Cash"]["0528.img"][string_format::extend_id(item_id, 8)]["emotion"];
		int32_t total = 0;

		for (nl::node face : faces)
			total += face.get_integer();

		if (total <= 0)
			return;

		int32_t roll = Randomizer().next_int(total);
		int32_t emotion = -1;

		for (nl::node face : faces)
		{
			roll -= static_cast<int32_t>(face.get_integer());

			if (roll < 0)
			{
				emotion = Expression::emotion(face.name());
				break;
			}
		}

		// Not morphed, and no faster than the emote keys (FUN_00a24470).
		if (emotion < 0 || player.shape_refuses() || !emote_cooldown.try_start(get_uptime() / 1000))
			return;

		player.set_expression(99 + emotion);
		FaceExpressionPacket(emotion).dispatch();
	}

	void Stage::use_item_effect(int32_t item_id)
	{
		// Not morphed or a ghost (FUN_00a24530).
		if (player.shape_refuses() || player.ghost_refuses())
			return;

		int32_t next = item_effect::toggled(player.get_item_effect(), item_id);
		int64_t now = get_uptime() / 1000;

		if (!item_effect::may_send(next, now, item_effect_shown))
			return;

		if (next != 0)
			item_effect_shown = now;

		// The player sees it at once; the server shows it to the others.
		player.set_item_effect(next);
		UseItemEffectPacket(next).dispatch();
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
		// StringPool 0xf52.
		const char *const TAKING_QUIZ = "You can't do this while taking the quiz.";
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
	// check that the player is alive. A quiz keeps the player out.
	void Stage::enter_cash_shop()
	{
		if (player.get_look().get_alerted())
		{
			notice(CATCH_BREATH);
			return;
		}

		if (!player.can_request(ExclRequest::WAIT, true))
			return;

		if (taking_quiz())
		{
			if (auto chatbar = UI::get().get_element<UIChatBar>())
				chatbar->send_chatline(TAKING_QUIZ, UIChatBar::LineType::SKILL);

			return;
		}

		if (!mapinfo.can_migrate())
		{
			notice(NOT_HERE);
			return;
		}

		player.request(EnterCashShopPacket(), ExclRequest::WAIT, true);
	}
}