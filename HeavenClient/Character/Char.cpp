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
#include "Char.h"

#include "Pets.h"

#include "../Data/WeaponData.h"
#include "../Net/Packets/GameplayPackets.h"
#include "../Util/Misc.h"

#include <nlnx/nx.hpp>

#include <algorithm>

namespace ms
{
	namespace
	{
		// The guild tag hangs 1 pixel under the name tag. The emblem sits to
		// its left, 1 pixel clear of the box, with their tops level
		// (FUN_005f0334). Not yet checked against period footage.
		constexpr int16_t GUILD_TAG_GAP = 1;
		constexpr Point<int16_t> EMBLEM_OFFSET = Point<int16_t>(-GuildMark::SIZE - 3, 6);

		// v83 hangs a room's bubble over the avatar's drawn height, hat and
		// all (FUN_0048e85a); this client doesn't measure it, so it takes a
		// bare head's.
		constexpr int16_t AVATAR_HEIGHT = 70;
	}

	Char::Char(int32_t o, const CharLook &lk, const std::string &name) : MapObject(o), look(lk), look_preview(lk), namelabel(
			Text(Text::Font::A13M, Text::Alignment::CENTER, Color::Name::WHITE, Text::Background::NAMETAG, name)),
			guildlabel(Text(Text::Font::A13M, Text::Alignment::CENTER, Color::Name::WHITE, Text::Background::NAMETAG))
	{}

	void Char::draw(double viewx, double viewy, float alpha) const
	{
		Point<int16_t> absp = phobj.get_absolute(viewx, viewy, alpha);

		effects.drawbelow(absp, alpha);
		held_effects.drawbelow(absp, alpha);

		Color color;

		// While blinking, the avatar is drawn at half brightness for two of
		// every four 30 ms updates (FUN_00930b27).
		if (invincible && blinking && invincible.elapsed() / 30 % 4 < 2)
		{
			color = Color(0.5f, 0.5f, 0.5f, 1.0f);
		} else
		{
			color = Color::Code::CWHITE;
		}

		if (state == State::DIED)
		{
			// The ghost floats in front of its tombstone, circling it from the top.
			float falling = std::max(TOMB_FALL - (tomb_tick + alpha), 0.0f);
			int16_t drop = static_cast<int16_t>(std::lround(TOMB_DROP * falling / TOMB_FALL));
			tomb.draw(absp - Point<int16_t>(0, drop), alpha);
			color = color * Color(1.0f, 1.0f, 1.0f, ghost_opacity.get(alpha));

			float angle = 6.2831853f * (ghost_tick + alpha) / GHOST_LAP;
			absp = absp + Point<int16_t>(
				static_cast<int16_t>(std::lround(GHOST_RADIUS * std::sin(angle))),
				static_cast<int16_t>(std::lround(-GHOST_RISE - GHOST_RADIUS * std::cos(angle))));
		}

		bool on_chair = chair && state == State::SIT;
		Point<int16_t> face = look.get_facepos();
		Point<int16_t> facepos = absp + Point<int16_t>(facing_right ? -face.x() : face.x(), face.y());
		Point<int16_t> chairpos = on_chair && chair_at_face ? facepos : absp;

		if (on_chair && !chair_in_front)
			chair_effect.draw(DrawArgument(chairpos, facing_right), alpha);

		look.draw(DrawArgument(absp, color), alpha);

		if (on_chair && chair_in_front)
			chair_effect.draw(DrawArgument(chairpos, facing_right), alpha);

		afterimage.draw(look.get_frame(), DrawArgument(absp, facing_right), alpha);

		if (ironbody)
		{
			float ibalpha = ironbody.alpha();
			float scale = 1.0f + ibalpha;
			float opacity = 1.0f - ibalpha;

			look.draw(DrawArgument(absp, scale, scale, opacity), alpha);
		}

		for (auto &pet : pets)
			if (pet.get_itemid())
				pet.draw(viewx, viewy, alpha);

		// If ever changing code for namelabel confirm placements with map 10000
		namelabel.draw(absp + Point<int16_t>(0, -4));

		if (!guildlabel.empty())
		{
			Point<int16_t> guildpos = absp + Point<int16_t>(0, -4 + namelabel.height() - 2 + GUILD_TAG_GAP);

			guildlabel.draw(guildpos);
			guildmark.draw(guildpos + Point<int16_t>(-guildlabel.width() / 2, 0) + EMBLEM_OFFSET);
		}

		// The orbs are in front of the avatar and its name tag (z 4 to the
		// tag's 2).
		draw_combo(absp);

		chalkboard.draw(absp, AVATAR_HEIGHT);
		chatballoon.draw(absp - Point<int16_t>(0, 85));
		hintballoon.draw(absp - Point<int16_t>(0, 85));
		// v83 puts a room's balloon in front of the chat balloon.
		miniroom.draw(absp, AVATAR_HEIGHT);

		effects.drawabove(absp, alpha);
		held_effects.drawabove(absp, alpha);

		for (auto &[reason, affliction] : afflictions)
			affliction.animation.draw(affliction.at_face ? facepos : absp, alpha);

		for (auto &number : damagenumbers)
			number.draw(viewx, viewy, alpha);
	}

	void Char::draw_combo(Point<int16_t> position) const
	{
		using combo::Orbs;

		Point<int16_t> center = position - Point<int16_t>(0, Orbs::RISE);

		for (size_t advanced = 0; advanced < 2; advanced++)
			if (combo_orbs.shown(advanced))
				combo_sprites[advanced][0].draw(
					DrawArgument(combo_orbs.center_angle(), center, combo_orbs.opacity(advanced)));

		float cos = std::cos(combo_orbs.ring_angle());
		float sin = std::sin(combo_orbs.ring_angle());

		for (int32_t j = 1; j <= Orbs::MOST; j++)
		{
			size_t layer = Orbs::orb(j);

			if (!combo_orbs.shown(layer))
				continue;

			Orbs::Offset slot = Orbs::slot(j);
			Point<int16_t> pos = center + Point<int16_t>(
				static_cast<int16_t>(std::lround(slot.x * cos - slot.y * sin)),
				static_cast<int16_t>(std::lround(slot.x * sin + slot.y * cos)));
			float scale = combo_orbs.scale(layer);

			combo_sprites[Orbs::advanced(j)][Orbs::state(j)].draw(
				DrawArgument(pos, scale, scale, combo_orbs.opacity(layer)));
		}
	}

	void Char::draw_preview(Point<int16_t> position, float alpha) const
	{
		look_preview.draw(position, false, Stance::Id::STAND1, Expression::Id::DEFAULT);
	}

	bool Char::update(const Physics& physics, float speed)
	{
		damagenumbers.remove_if(
				[](DamageNumber &number)
				{
					return number.update();
				}
		);

		effects.update();
		held_effects.update();
		chair_effect.update();

		for (auto &[reason, affliction] : afflictions)
			affliction.animation.update();

		chatballoon.update();
		hintballoon.update();
		combo_orbs.update(Constants::TIMESTEP);
		invincible.update();
		ironbody.update();

		if (state == State::DIED)
		{
			if (tomb_tick < TOMB_FALL)
				tomb_tick++;

			ghost_opacity += GHOST_FADE;
			ghost_tick = (ghost_tick + 1) % GHOST_LAP;

			if (ghost_opacity.last() > 1.0f - GHOST_FADE)
				ghost_opacity.set(1.0f);

			if (!tomb_landed && tomb.update())
			{
				tomb = tomb_land;
				tomb_landed = true;
			}
		}

		for (size_t i = 0; i < pets.size(); i++)
		{
			PetLook &pet = pets[i];

			if (!pet.get_itemid())
				continue;

			pet.set_wear(look.get_pet_wear(i));

			// Another's pet goes where its owner's client says.
			if (pet.is_local())
			{
				switch (state)
				{
					case State::LADDER:
					case State::ROPE:
						pet.set_stance(PetLook::Stance::HANG);
						break;
					case State::SWIM:
						pet.set_stance(PetLook::Stance::FLY);
						break;
					default:
						if (pet.get_stance() == PetLook::Stance::HANG || pet.get_stance() == PetLook::Stance::FLY)
							pet.set_stance(PetLook::Stance::STAND);

						break;
				}
			}

			if (std::optional<move_path::Path> path = pet.update(physics, get_position()))
				MovePetPacket(pet.get_serial(), *path).dispatch();
		}

		uint16_t stancespeed = 0;

		if (speed >= 1.0f / Constants::TIMESTEP)
		{
			// Keep the fraction, or a speed of 16/14 would step 9 ms and
			// play no faster than 9/8.
			float step = Constants::TIMESTEP * speed + stance_carry;
			stancespeed = static_cast<uint16_t>(step);
			stance_carry = step - stancespeed;
		} else
		{
			stance_carry = 0.0f;
		}

		afterimage.update(look.get_frame(), stancespeed);

		return look.update(stancespeed);
	}

	float Char::get_stancespeed() const
	{
		if (attacking)
			return get_real_attackspeed();

		switch (state)
		{
			case State::WALK:
				// v83 paces each walk frame to delay * 100 / speed, holding the
				// speed to 70..140, however fast the body moves (FUN_00453ad1).
				return std::clamp<int32_t>(get_walk_animation_speed(), 70, 140) / 100.0f;
			case State::LADDER:
			case State::ROPE:
				// Climbing plays at its own pace, and holds still between moves.
				return phobj.vspeed != 0.0 ? 1.0f : 0.0f;
			default:
				return 1.0f;
		}
	}

	int32_t Char::get_walk_animation_speed() const
	{
		return 100;
	}

	float Char::get_real_attackspeed() const
	{
		return 16.0f / (get_action_speed() + 10);
	}

	int8_t Char::get_action_speed() const
	{
		// v83 stretches each frame of an action to delay * (speed + 10) / 16,
		// holding the speed to 2..10 (FUN_00453ad1).
		if (fixed_action_speed > 0)
			return fixed_action_speed;

		return std::clamp<int8_t>(get_integer_attackspeed(), 2, 10);
	}

	uint16_t Char::get_attackdelay(size_t no) const
	{
		uint8_t first_frame = afterimage.get_first_frame();
		uint16_t delay = look.get_attackdelay(no, first_frame);

		return static_cast<uint16_t>(delay * (get_action_speed() + 10) / 16);
	}

	uint16_t Char::get_attack_length() const
	{
		return static_cast<uint16_t>(look.get_attack_length() * (get_action_speed() + 10) / 16);
	}

	int32_t Char::get_action_duration(const std::string &action) const
	{
		int32_t duration = 0;

		for (uint16_t delay : look.get_action_delays(action))
			duration += delay * (get_action_speed() + 10) / 16;

		return duration;
	}

	uint16_t Char::get_action_lock(const std::string &action) const
	{
		return look.get_action_lock(action);
	}

	int8_t Char::update(const Physics &physics)
	{
		update(physics, 1.0f);

		return get_layer();
	}

	int8_t Char::get_layer() const
	{
		return is_climbing() ? 7 : phobj.fhlayer;
	}

	void Char::show_attack_effect(Animation toshow, int8_t z)
	{
		float attackspeed = get_real_attackspeed();

		effects.add(toshow, DrawArgument(facing_right), z, attackspeed);
	}

	void Char::show_combo(int32_t orbs, bool init)
	{
		combo_orbs.set(orbs, init);
	}

	void Char::show_combo_finish(int32_t skill)
	{
		int32_t orbs = combo_orbs.get_count();

		if (orbs < 1 || orbs > combo::Orbs::MOST)
			return;

		nl::node finish = nl::nx::skill[string_format::skill_img(skill)]["skill"][std::to_string(skill)]["finish"];
		effects.add(finish[std::to_string(orbs)], DrawArgument(false), static_cast<int8_t>(finish["z"].get_integer()),
			get_real_attackspeed());
	}

	void Char::show_held_effect(Animation toshow, int8_t z, bool looping)
	{
		float attackspeed = get_real_attackspeed();

		if (looping)
			held_effects.add_looping(toshow, DrawArgument(facing_right), z, attackspeed);
		else
			held_effects.add(toshow, DrawArgument(facing_right), z, attackspeed);
	}

	void Char::clear_held_effects()
	{
		held_effects.clear();
	}

	void Char::show_afflictions(const std::vector<int32_t> &reasons)
	{
		for (auto iter = afflictions.begin(); iter != afflictions.end();)
		{
			if (std::find(reasons.begin(), reasons.end(), iter->first) == reasons.end())
				iter = afflictions.erase(iter);
			else
				++iter;
		}

		for (int32_t reason : reasons)
		{
			if (afflictions.count(reason))
				continue;

			nl::node affected = nl::nx::skill["MobSkill.img"][std::to_string(reason & 0xFFFF)]["level"]
			[std::to_string(reason >> 16)]["affected"];

			afflictions.emplace(reason, Affliction{ affected, affected["pos"].get_integer() == 1 });
		}
	}

	void Char::show_effect_id(CharEffect::Id toshow)
	{
		effects.add(chareffects[toshow]);
	}

	void Char::show_effect(const Animation &effect)
	{
		effects.add(effect);
	}

	void Char::show_skill_book(bool success)
	{
		effects.add(skill_book[success][0], DrawArgument(), 3);
		effects.add(skill_book[success][1], DrawArgument(), -1);
	}

	void Char::show_iron_body()
	{
		ironbody.set_for(500);
	}

	void Char::show_damage(int32_t damage)
	{
		// v83 anchors it on the top of the avatar's image; a bare head's top
		// is 64 px above the feet when standing.
		damagenumbers.emplace_back(DamageNumber::Type::TOPLAYER, damage);
		damagenumbers.back().set_anchor(Point<int16_t>(phobj.get_x(), phobj.get_y() - 64));

		look.set_alerted(5000);

		if (damage > 0)
			set_expression(Expression::Id::HIT, 1500);
	}

	void Char::become_invincible(bool blink)
	{
		invincible.set_for(1500);
		blinking = blink;
	}

	void Char::show_heal(int32_t amount)
	{
		int16_t start_y = phobj.get_y() - 60;
		int16_t x = phobj.get_x();

		damagenumbers.emplace_back(DamageNumber::Type::HEAL, amount);
		damagenumbers.back().set_anchor(Point<int16_t>(x, start_y));
	}

	void Char::speak(const std::string &line)
	{
		chatballoon.change_text(line);
	}

	void Char::show_hint(const std::string &text, int16_t width)
	{
		hintballoon = ChatBalloon("tutorial", true, static_cast<uint16_t>(std::max<int16_t>(width, 40)));
		hintballoon.change_text(text);
	}

	void Char::change_look(MapleStat::Id stat, int32_t id)
	{
		switch (stat)
		{
			case MapleStat::Id::SKIN:
				look.set_body(id);
				break;
			case MapleStat::Id::FACE:
				look.set_face(id);
				break;
			case MapleStat::Id::HAIR:
				look.set_hair(id);
				break;
		}
	}

	void Char::set_state(uint8_t statebyte)
	{
		if (statebyte % 2 == 1)
		{
			set_direction(false);

			statebyte -= 1;
		} else
		{
			set_direction(true);
		}

		Char::State newstate = by_value(statebyte);
		set_state(newstate);
	}

	void Char::set_expression(int32_t expid)
	{
		Expression::Id expression = Expression::byaction(expid);
		look.set_expression(expression);
	}

	void Char::set_expression(Expression::Id expression, int32_t hold)
	{
		look.set_expression(expression, hold);
	}

	void Char::attack(const std::string &action)
	{
		look.set_action(action);

		attacking = true;
		fixed_action_speed = 0;
		look.set_alerted(5000);
	}

	void Char::attack(Stance::Id stance)
	{
		look.attack(stance);

		attacking = true;
		fixed_action_speed = 0;
		look.set_alerted(5000);
	}

	void Char::attack(bool degenerate)
	{
		look.attack(degenerate);

		attacking = true;
		fixed_action_speed = 0;
		look.set_alerted(5000);
	}

	void Char::attack(bool degenerate, Stance::Id stance)
	{
		look.attack(degenerate, stance);

		attacking = true;
		fixed_action_speed = 0;
		look.set_alerted(5000);
	}

	void Char::hold_attack(uint8_t frame)
	{
		look.hold(frame);
	}

	void Char::fix_action_speed(int8_t speed)
	{
		fixed_action_speed = speed;
	}

	void Char::end_attack()
	{
		look.end_attack();

		attacking = false;
	}

	void Char::set_afterimage(int32_t skill_id)
	{
		int32_t weapon_id = look.get_equips().get_weapon();

		if (weapon_id <= 0)
			return;

		const WeaponData &weapon = WeaponData::get(weapon_id);

		std::string stance_name = Stance::names[look.get_stance()];
		int16_t weapon_level = weapon.get_equipdata().get_reqstat(MapleStat::Id::LEVEL);
		const std::string& ai_name = weapon.get_afterimage();

		afterimage = Afterimage(skill_id, ai_name, stance_name, weapon_level);
	}

	const Afterimage &Char::get_afterimage() const
	{
		return afterimage;
	}

	Rectangle<int16_t> Char::get_afterimage_range(Stance::Id stance) const
	{
		int32_t weapon_id = look.get_equips().get_weapon();

		if (weapon_id <= 0)
			return {};

		const WeaponData &weapon = WeaponData::get(weapon_id);
		int16_t weapon_level = weapon.get_equipdata().get_reqstat(MapleStat::Id::LEVEL);
		nl::node src = nl::nx::character["Afterimage"][weapon.get_afterimage() + ".img"];

		return Rectangle<int16_t>(src[weapon_level / 10][Stance::names[stance]]);
	}

	void Char::set_direction(bool f)
	{
		facing_right = f;
		look.set_direction(f);
	}

	void Char::set_state(State st)
	{
		if (st == State::DIED && state != State::DIED)
		{
			tomb = tomb_fall;
			tomb_tick = 0;
			tomb_landed = false;
			ghost_opacity.set(0.0f);
			ghost_tick = 0;
		}

		state = st;

		Stance::Id stance = Stance::by_state(state);
		look.set_stance(stance);
	}

	void Char::add_pet(uint8_t index, bool replace, const PetEntry &pet, bool local)
	{
		if (replace)
			pets::remove(pets, index);

		pets::insert(pets, index, PetLook(pet, local));
	}

	void Char::remove_pet(uint8_t index)
	{
		pets::remove(pets, index);
	}

	void Char::clear_pets()
	{
		pets = {};
	}

	std::array<int64_t, 3> Char::get_pet_serials() const
	{
		std::array<int64_t, 3> serials{};

		for (size_t i = 0; i < pets.size(); i++)
			serials[i] = pets[i].get_itemid() ? pets[i].get_serial() : 0;

		return serials;
	}

	void Char::send_pet_movement(uint8_t index, const std::vector<Movement> &movements)
	{
		if (Optional<PetLook> pet = get_pet(index))
			pet->send_movement(movements);
	}

	Optional<PetLook> Char::get_pet(uint8_t index)
	{
		if (index < pets.size() && pets[index].get_itemid())
			return pets[index];

		return {};
	}

	bool Char::is_invincible() const
	{
		return invincible == true;
	}

	bool Char::is_sitting() const
	{
		return state == State::SIT;
	}

	bool Char::is_prone() const
	{
		return state == State::PRONE;
	}

	bool Char::is_dead() const
	{
		return state == State::DIED;
	}

	void Char::set_chair(int32_t itemid)
	{
		if (itemid == chair)
			return;

		chair = itemid;

		if (!chair)
			return;

		nl::node src = nl::nx::item["Install"]["0301.img"][string_format::extend_id(itemid, 8)]["effect"];
		chair_effect = src;
		chair_at_face = src["pos"].get_integer() == 1;
		chair_in_front = src["z"].get_integer() >= 0;
	}

	int32_t Char::get_chair() const
	{
		return chair;
	}

	bool Char::is_climbing() const
	{
		return state == State::LADDER || state == State::ROPE;
	}

	bool Char::is_twohanded() const
	{
		return look.get_equips().is_twohanded();
	}

	Weapon::Type Char::get_weapontype() const
	{
		int32_t weapon_id = look.get_equips().get_weapon();

		if (weapon_id <= 0)
			return Weapon::Type::NONE;

		return WeaponData::get(weapon_id).get_type();
	}

	bool Char::getflip() const
	{
		return facing_right;
	}

	std::string Char::get_name() const
	{
		return namelabel.get_text();
	}

	void Char::set_guild_name(const std::string &name)
	{
		guildlabel.change_text(name);
	}

	void Char::set_guild_emblem(const GuildEmblem &emblem)
	{
		guildmark = GuildMark(emblem);
	}

	void Char::set_mini_room(const std::optional<MiniRoomBox> &box)
	{
		miniroom.change(box);
	}

	int32_t Char::mini_room_at(Point<int16_t> feet, Point<int16_t> cursor) const
	{
		return miniroom.serial_at(feet, AVATAR_HEIGHT, cursor);
	}

	void Char::set_chalkboard(const std::optional<std::string> &text, bool closable)
	{
		chalkboard.change(text, closable);
	}

	std::optional<std::string> Char::get_chalkboard() const
	{
		if (!chalkboard.is_shown())
			return std::nullopt;

		return chalkboard.get_text();
	}

	bool Char::send_chalkboard_cursor(Point<int16_t> feet, Point<int16_t> cursor, bool pressed, bool held)
	{
		return chalkboard.send_cursor(feet, AVATAR_HEIGHT, cursor, pressed, held);
	}

	void Char::release_chalkboard_cursor()
	{
		chalkboard.release_cursor();
	}

	bool Char::on_chalkboard_close(Point<int16_t> feet, Point<int16_t> cursor) const
	{
		return chalkboard.on_close(feet, AVATAR_HEIGHT, cursor);
	}

	CharLook &Char::get_look()
	{
		return look;
	}

	const CharLook &Char::get_look() const
	{
		return look;
	}

	PhysicsObject &Char::get_phobj()
	{
		return phobj;
	}

	const PhysicsObject &Char::get_phobj() const
	{
		return phobj;
	}

	void Char::init()
	{
		CharLook::init();

		nl::node src = nl::nx::effect["BasicEff.img"];

		for (auto iter : CharEffect::PATHS)
			chareffects.emplace(iter.first, src.resolve(iter.second));

		for (int success = 0; success < 2; success++)
			for (int layer = 0; layer < 2; layer++)
				skill_book[success][layer] = src["SkillBook"][success ? "Success" : "Failure"][std::to_string(layer)];

		nl::node tomb = nl::nx::effect["Tomb.img"];
		tomb_fall = tomb["fall"];
		tomb_land = tomb["land"];

		// Dawn Warriors show these too: v83 only ever loads the Hero's.
		nl::node combo = nl::nx::skill["111.img"]["skill"]["1111002"]["state"];
		nl::node advanced = nl::nx::skill["112.img"]["skill"]["1120003"]["state"];

		for (int state = 0; state < 6; state++)
		{
			combo_sprites[0][state] = combo[std::to_string(state)];
			combo_sprites[1][state] = advanced[std::to_string(state)];
		}
	}

	EnumMap<CharEffect::Id, Animation> Char::chareffects;
	Animation Char::tomb_fall;
	Animation Char::tomb_land;
	Animation Char::skill_book[2][2];
	Texture Char::combo_sprites[2][6];
}