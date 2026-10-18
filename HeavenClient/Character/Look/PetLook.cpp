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

#include <nlnx/nx.hpp>

namespace ms
{
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
															   entry.fh, entry.stance })
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
						 shown({ 0.0, 0.0, 0.0, 0.0, 0, 0 })
	{}

	void PetLook::draw(double viewx, double viewy, float alpha) const
	{
		Point<int16_t> absp = phobj.get_absolute(viewx, viewy, alpha);

		animations[stance].draw(DrawArgument(absp, flip), alpha);
		namelabel.draw(absp);
	}

	std::optional<move_path::Path> PetLook::update(const Physics &physics, Point<int16_t> charpos)
	{
		static const double PETWALKFORCE = 0.35;
		static const double PETFLYFORCE = 0.2;

		if (!local)
		{
			shown.step(Constants::TIMESTEP);

			const move_path::Player::State &now = shown.get_state();
			set_stance(now.stance);
			phobj.hspeed = now.x - phobj.crnt_x();
			phobj.vspeed = now.y - phobj.crnt_y();
			phobj.move();

			animations[stance].update();

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

	void PetLook::send_movement(const std::vector<Movement> &movements)
	{
		shown.receive(movements);
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