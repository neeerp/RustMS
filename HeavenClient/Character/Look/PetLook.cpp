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
#include "PetLook.h"

#include "../Pets.h"

#include "../../Constants.h"
#include "../../Audio/Audio.h"
#include "../../Data/PetData.h"
#include "../../Util/Randomizer.h"

#include <nlnx/nx.hpp>

namespace ms
{
	namespace
	{
		// One of the answers in `list` and one of its lines, if it has any
		// (FUN_007048ab, FUN_00704a92).
		PetLook::Said pick(uint8_t type, const std::vector<pets::Reaction> &list,
						   const std::vector<std::string> &customs)
		{
			Randomizer random;
			const pets::Reaction &reaction = list[random.next_int(list.size())];
			std::string text = reaction.lines.empty() ? "" : reaction.lines[random.next_int(reaction.lines.size())];

			return { type, pets::action(reaction.act, customs), text };
		}
	}

	PetLook::Stance PetLook::stancebyvalue(uint8_t value)
	{
		switch (pets::shown(value))
		{
			case 0:
				return Stance::MOVE;
			case 3:
				return Stance::JUMP;
			case 4:
				return Stance::FLY;
			case 8:
				return Stance::HANG;
			default:
				// stand1, hungry and rest show as standing.
				return Stance::STAND;
		}
	}

	PetLook::PetLook(const PetEntry &entry, bool lc) : itemid(entry.itemid), name(entry.name), serial(entry.serial),
													   local(lc), stance(Stance::STAND), flip(false),
													   shown({ static_cast<double>(entry.position.x()),
															   static_cast<double>(entry.position.y()), 0.0, 0.0,
															   entry.fh, entry.stance }),
													   action(-1), effecting(false), pending(false), still(0)
	{
		set_position(entry.position.x(), entry.position.y());
		set_stance(entry.stance);
		recorder.reset(Movement(phobj, get_stancebyte()));

		namelabel = Text(Text::Font::A13M, Text::Alignment::CENTER, Color::Name::WHITE, Text::Background::NAMETAG,
						 name);

		std::string strid = std::to_string(itemid);
		nl::node src = nl::nx::item["Pet"][strid + ".img"];

		animations[Stance::MOVE] = src["move"];
		animations[Stance::STAND] = src["stand0"];
		animations[Stance::JUMP] = src["jump"];
		animations[Stance::ALERT] = src["alert"];
		animations[Stance::PRONE] = src["prone"];
		animations[Stance::FLY] = src["fly"];
		animations[Stance::HANG] = src["hang"];

		nl::node effsrc = nl::nx::effect["PetEff.img"][strid];

		animations[Stance::WARP] = effsrc["warp"];
	}

	PetLook::PetLook() : itemid(0), serial(0), local(false), stance(Stance::STAND), flip(false),
						 shown({ 0.0, 0.0, 0.0, 0.0, 0, 0 }), action(-1), effecting(false), pending(false),
						 still(0)
	{}

	void PetLook::draw(double viewx, double viewy, float alpha) const
	{
		Point<int16_t> absp = phobj.get_absolute(viewx, viewy, alpha);

		if (action >= 0)
			acting.draw(DrawArgument(absp, flip), alpha);
		else
			animations[stance].draw(DrawArgument(absp, flip), alpha);

		if (effecting)
			effect.draw(DrawArgument(absp), alpha);

		namelabel.draw(absp);
		balloon.draw(absp - Point<int16_t>(0, animations[Stance::STAND].get_origin().y()));
	}

	std::optional<move_path::Path> PetLook::update(const Physics &physics, Point<int16_t> charpos)
	{
		static const double PETWALKFORCE = 0.35;
		static const double PETFLYFORCE = 0.2;

		Point<int16_t> before = phobj.get_position();

		if (!local)
		{
			shown.step(Constants::TIMESTEP);

			const move_path::Player::State &now = shown.get_state();
			set_stance(now.stance);
			phobj.hspeed = now.x - phobj.crnt_x();
			phobj.vspeed = now.y - phobj.crnt_y();
			phobj.move();

			animations[stance].update();
			update_acting(before);

			return std::nullopt;
		}

		Point<int16_t> curpos = phobj.get_position();
		bool teleported = false;

		switch (stance)
		{
			case Stance::STAND:
			case Stance::MOVE:
				if (curpos.distance(charpos) > 150)
				{
					set_position(charpos.x(), charpos.y());
					teleported = true;
				} else
				{
					if (charpos.x() - curpos.x() > 50)
					{
						phobj.hforce = PETWALKFORCE;
						flip = true;

						set_stance(Stance::MOVE);
					} else if (charpos.x() - curpos.x() < -50)
					{
						phobj.hforce = -PETWALKFORCE;
						flip = false;

						set_stance(Stance::MOVE);
					} else
					{
						phobj.hforce = 0.0;

						set_stance(Stance::STAND);
					}
				}

				phobj.type = PhysicsObject::Type::NORMAL;
				phobj.clear_flag(PhysicsObject::Flag::NOGRAVITY);
				break;
			case Stance::HANG:
				set_position(charpos.x(), charpos.y());
				phobj.set_flag(PhysicsObject::Flag::NOGRAVITY);
				break;
			case Stance::FLY:
				if ((charpos - curpos).length() > 250)
				{
					set_position(charpos.x(), charpos.y());
					teleported = true;
				} else
				{
					if (charpos.x() - curpos.x() > 50)
					{
						phobj.hforce = PETFLYFORCE;
						flip = true;
					} else if (charpos.x() - curpos.x() < -50)
					{
						phobj.hforce = -PETFLYFORCE;
						flip = false;
					} else
					{
						phobj.hforce = 0.0f;
					}

					if (charpos.y() - curpos.y() > 50.0f)
						phobj.vforce = PETFLYFORCE;
					else if (charpos.y() - curpos.y() < -50.0f)
						phobj.vforce = -PETFLYFORCE;
					else
						phobj.vforce = 0.0f;
				}

				phobj.type = PhysicsObject::Type::FLYING;
				phobj.clear_flag(PhysicsObject::Flag::NOGRAVITY);
				break;
		}

		physics.move_object(phobj);

		animations[stance].update();
		update_acting(before);

		// A pet left behind comes back by teleport (4), as a summon does.
		if (teleported)
			recorder.add(Movement(phobj, get_stancebyte(), 4));
		else
			recorder.step(Movement(phobj, get_stancebyte()), Constants::TIMESTEP);

		bool flying = stance == Stance::FLY;

		if (recorder.ready(flying))
		{
			move_path::Path path = recorder.flush(flying);

			if (!path.idle)
				return path;
		}

		return std::nullopt;
	}

	void PetLook::update_acting(Point<int16_t> before)
	{
		balloon.update();

		if (effecting && effect.update())
			effecting = false;

		if (phobj.get_position() != before)
		{
			still = 0;
			action = -1;

			return;
		}

		still += Constants::TIMESTEP;

		if (action >= 0)
		{
			// Its own animations play once; a base one holds until it moves.
			if (acting.update() && action >= static_cast<int16_t>(pets::BASE_ACTIONS.size()))
				action = -1;
		}
		else if (pets::idle_due(still, Constants::TIMESTEP) && Randomizer().next_bool())
		{
			const PetData &data = PetData::get(itemid);

			if (!data.get_idle().empty())
				act(pick(3, data.get_idle(), data.get_customs()), false);
		}
	}

	void PetLook::send_movement(const std::vector<Movement> &movements)
	{
		shown.receive(movements);
	}

	bool PetLook::act(const Said &said, bool styled)
	{
		if (said.action < 0)
			return false;

		const PetData &data = PetData::get(itemid);

		// A hanging pet keeps hanging (any action becomes 8). HC shows no
		// walking (0) on a pet standing still.
		if (stance != Stance::HANG && said.action > 0)
		{
			std::string strid = std::to_string(itemid);
			std::string actname = pets::action_name(said.action, data.get_customs());
			nl::node src = nl::nx::item["Pet"][strid + ".img"][actname];

			if (src.size() > 0)
			{
				acting = src;
				action = said.action;
			}

			// A command's answer plays the pet's sound for its own actions.
			if (said.type == 0 && said.action >= static_cast<int16_t>(pets::BASE_ACTIONS.size()))
				Sound(nl::nx::sound["Pet.img"][strid][actname]).play();
		}

		if (!said.text.empty())
		{
			balloon = ChatBalloon("pet/" + std::to_string(styled ? data.get_balloon() : 0), false);
			balloon.change_text(said.text);
		}

		return true;
	}

	std::optional<pets::Heard> PetLook::hear(const std::string &line, int16_t level)
	{
		if (!local || pending || still < 500 || action >= static_cast<int16_t>(pets::BASE_ACTIONS.size()))
			return std::nullopt;

		const PetData &data = PetData::get(itemid);
		std::optional<pets::Heard> heard = pets::hear(line, name, data.get_kind(), level, data.get_commands());

		if (heard)
			pending = true;

		return heard;
	}

	std::optional<PetLook::Said> PetLook::answer(uint8_t command, bool obeyed, bool styled, int16_t level)
	{
		pending = false;

		const PetData &data = PetData::get(itemid);
		const std::vector<pets::Command> &commands = data.get_commands();

		if (command >= commands.size() || level < commands[command].min || level > commands[command].max)
			return std::nullopt;

		const std::vector<pets::Reaction> &list = obeyed ? commands[command].success : commands[command].fail;

		if (list.empty())
			return std::nullopt;

		Said said = pick(0, list, data.get_customs());

		if (!act(said, styled))
			return std::nullopt;

		return said;
	}

	std::optional<PetLook::Said> PetLook::eat(bool enjoyed, bool styled, int16_t level)
	{
		const PetData &data = PetData::get(itemid);
		const pets::Command *food = pets::food_for(data.get_food(), level);

		if (!food)
			return std::nullopt;

		const std::vector<pets::Reaction> &list = enjoyed ? food->success : food->fail;

		if (list.empty())
			return std::nullopt;

		Said said = pick(1, list, data.get_customs());

		if (!act(said, styled))
			return std::nullopt;

		return said;
	}

	void PetLook::show_effect(uint8_t kind)
	{
		nl::node src = nl::nx::effect["PetEff.img"];

		switch (kind)
		{
			case 0:
				effect = src["Basic"]["LevelUp"];
				Sound(Sound::Name::LEVELUP).play();
				break;
			case 1:
				effect = src["Basic"]["Teleport"];
				break;
			case 2:
				effect = src[std::to_string(itemid)]["warp"];
				break;
			case 3:
				effect = src["Basic"]["Evolution"];
				break;
			default:
				return;
		}

		effecting = true;
	}

	void PetLook::set_position(int16_t x, int16_t y)
	{
		phobj.set_x(x);
		phobj.set_y(y);
	}

	void PetLook::set_stance(Stance st)
	{
		if (stance != st)
		{
			stance = st;
			animations[stance].reset();
		}
	}

	void PetLook::set_stance(uint8_t stancebyte)
	{
		flip = stancebyte % 2 == 0;
		set_stance(stancebyvalue(stancebyte));
	}

	uint8_t PetLook::get_stancebyte() const
	{
		uint8_t action;

		switch (stance)
		{
			case Stance::MOVE:
				action = 1;
				break;
			case Stance::JUMP:
				action = 3;
				break;
			case Stance::FLY:
				action = 6;
				break;
			case Stance::HANG:
				action = 15;
				break;
			default:
				action = 2;
				break;
		}

		return static_cast<uint8_t>(action << 1 | (flip ? 0 : 1));
	}

	void PetLook::play_eat_sound() const
	{
		Sound(nl::nx::sound["Pet.img"][std::to_string(itemid)]["eat"]).play();
	}

	int32_t PetLook::get_itemid() const
	{
		return itemid;
	}

	int64_t PetLook::get_serial() const
	{
		return serial;
	}

	bool PetLook::is_local() const
	{
		return local;
	}

	PetLook::Stance PetLook::get_stance() const
	{
		return stance;
	}
}