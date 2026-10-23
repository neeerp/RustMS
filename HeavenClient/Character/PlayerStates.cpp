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
#include "PlayerStates.h"

#include "../Gameplay/MapleMap/FieldLimit.h"
#include "../Gameplay/Stage.h"

namespace ms
{
	// Jumping off a rope or ladder in CVecCtrl::Jump (FUN_007f1bb0): 130% of
	// the walking speed sideways and half the takeoff speed upwards.
	const double ROPEJUMPH = 1.3;
	const double ROPEJUMPV = 0.5;
	// FUN_009b1d3d: off a rope in a swim map.
	const double ROPEJUMPSWIM = 0.3;

	// Climbing in CUserLocal (FUN_009cfdb1): 3 px each 30 ms update,
	// whatever the speed.
	const double CLIMBSPEED = 3.0 * Constants::TIMESTEP / 30;

	// Base class
	void PlayerState::play_jumpsound() const
	{
		Sound(Sound::JUMP).play();
	}

	Char::State PlayerState::ground_state(Player &player) const
	{
		int x = player.is_key_down(KeyAction::Id::RIGHT) - player.is_key_down(KeyAction::Id::LEFT);
		int y = player.is_key_down(KeyAction::Id::DOWN) - player.is_key_down(KeyAction::Id::UP);

		if (x != 0)
		{
			if (!player.is_attacking())
				player.set_direction(x > 0);

			return Char::State::WALK;
		}

		return y > 0 ? Char::State::PRONE : Char::State::STAND;
	}

	void PlayerState::stand_by_input(Player &player, Char::State current) const
	{
		Char::State next = ground_state(player);

		if (next != current)
			player.set_state(next);
	}

	void PlayerState::ground_jump(Player &player, int8_t dir) const
	{
		bool down = player.is_key_down(KeyAction::Id::DOWN);

		if (!field_limit::may_jump(Stage::get().get_mapinfo().get_fieldlimit(), down))
			return;

		if (down)
		{
			if (player.get_phobj().enablejd)
			{
				play_jumpsound();
				Physics::jump_down(player.get_phobj());
				player.set_state(Char::State::FALL);
			}

			return;
		}

		play_jumpsound();
		Physics::jump(player.get_phobj(), player.get_jumpforce(), dir);
	}

	// Null state
	void PlayerNullState::update_state(Player &player) const
	{
		Char::State state;

		if (player.get_phobj().onground)
		{
			state = ground_state(player);
		} else
		{
			Optional<const Ladder> ladder = player.get_ladder();

			if (ladder)
				state = ladder->is_ladder() ? Char::State::LADDER : Char::State::ROPE;
			else
				state = Char::State::FALL;
		}

		player.get_phobj().clear_flags();

		player.set_state(state);
	}

	// Standing
	void PlayerStandState::initialize(Player &player) const
	{
		player.get_phobj().type = PhysicsObject::Type::NORMAL;
	}

	void PlayerStandState::send_action(Player &player, KeyAction::Id ka, bool down) const
	{
		if (player.is_attacking())
			return;

		if (down && ka == KeyAction::Id::JUMP)
			ground_jump(player, 0);
	}

	void PlayerStandState::update(Player &player) const
	{
		if (player.get_phobj().enablejd == false)
			player.get_phobj().set_flag(PhysicsObject::Flag::CHECKBELOW);
	}

	void PlayerStandState::update_state(Player &player) const
	{
		if (player.get_phobj().onground)
			stand_by_input(player, Char::State::STAND);
		else
			player.set_state(Char::State::FALL);
	}

	// Walking
	void PlayerWalkState::initialize(Player &player) const
	{
		player.get_phobj().type = PhysicsObject::Type::NORMAL;
	}

	void PlayerWalkState::send_action(Player &player, KeyAction::Id ka, bool down) const
	{
		if (player.is_attacking())
			return;

		if (down && ka == KeyAction::Id::JUMP)
			ground_jump(player, haswalkinput(player) ? (player.getflip() ? 1 : -1) : 0);
	}

	bool PlayerWalkState::haswalkinput(const Player &player) const
	{
		return player.is_key_down(KeyAction::Id::LEFT) || player.is_key_down(KeyAction::Id::RIGHT);
	}

	void PlayerWalkState::update(Player &player) const
	{
		if (!player.is_attacking() && haswalkinput(player))
			player.get_phobj().walkdir = player.getflip() ? 1 : -1;

		if (player.get_phobj().enablejd == false)
			player.get_phobj().set_flag(PhysicsObject::Flag::CHECKBELOW);
	}

	void PlayerWalkState::update_state(Player &player) const
	{
		// A wall stopping the player does not end the walk.
		if (player.get_phobj().onground)
		{
			stand_by_input(player, Char::State::WALK);
		} else
		{
			player.set_state(Char::State::FALL);
		}
	}

	// Falling
	void PlayerFallState::initialize(Player &player) const
	{
		player.get_phobj().type = PhysicsObject::Type::NORMAL;
	}

	void PlayerFallState::send_action(Player &player, KeyAction::Id ka, bool down) const
	{
		if (down)
		{
			switch (ka)
			{
				case KeyAction::Id::LEFT:
					player.set_direction(false);
					break;
				case KeyAction::Id::RIGHT:
					player.set_direction(true);
					break;
			}
		}
	}

	void PlayerFallState::update(Player &player) const
	{
		if (player.is_key_down(KeyAction::Id::LEFT))
			player.get_phobj().walkdir = -1;
		else if (player.is_key_down(KeyAction::Id::RIGHT))
			player.get_phobj().walkdir = 1;
	}

	void PlayerFallState::update_state(Player &player) const
	{
		if (player.get_phobj().onground)
		{
			player.set_state(ground_state(player));
		} else if (player.is_underwater())
		{
			player.set_state(Char::State::SWIM);
		}
	}

	// Prone
	void PlayerProneState::send_action(Player &player, KeyAction::Id ka, bool down) const
	{
		if (down && ka == KeyAction::Id::JUMP)
			ground_jump(player, 0);
	}

	void PlayerProneState::update(Player &player) const
	{
		if (player.get_phobj().enablejd == false)
			player.get_phobj().set_flag(PhysicsObject::Flag::CHECKBELOW);
	}

	void PlayerProneState::update_state(Player &player) const
	{
		if (player.get_phobj().onground)
			stand_by_input(player, Char::State::PRONE);
	}

	// Sitting
	void PlayerSitState::send_action(Player &player, KeyAction::Id ka, bool down) const
	{
		// Getting off a portable chair waits for the server's CANCEL_CHAIR.
		if (player.get_chair())
		{
			if (down && (ka == KeyAction::Id::LEFT || ka == KeyAction::Id::RIGHT ||
						 ka == KeyAction::Id::JUMP || ka == KeyAction::Id::SIT))
				player.leave_chair();

			return;
		}

		if (down)
		{
			switch (ka)
			{
				case KeyAction::Id::LEFT:
					player.set_direction(false);
					player.set_state(Char::State::WALK);
					break;
				case KeyAction::Id::RIGHT:
					player.set_direction(true);
					player.set_state(Char::State::WALK);
					break;
				case KeyAction::Id::JUMP:
					play_jumpsound();
					player.set_state(Char::State::STAND);
					break;
				case KeyAction::Id::UP:
					player.set_state(Char::State::STAND);
					break;
			}
		}
	}

	// Flying
	void PlayerFlyState::initialize(Player &player) const
	{
		// Swimming is the shoe's swim step (r25 0x9b2c3c).
		player.get_phobj().type = player.is_underwater() ? PhysicsObject::Type::NORMAL : PhysicsObject::Type::FLYING;
	}

	void PlayerFlyState::send_action(Player &player, KeyAction::Id ka, bool down) const
	{
		if (down)
		{
			switch (ka)
			{
				case KeyAction::Id::LEFT:
					player.set_direction(false);
					break;
				case KeyAction::Id::RIGHT:
					player.set_direction(true);
					break;
				case KeyAction::Id::JUMP:
					if (player.is_underwater() && !player.is_attacking())
					{
						play_jumpsound();
						Physics::swim_jump(player.get_phobj());
					}

					break;
			}
		}
	}

	void PlayerFlyState::update(Player &player) const
	{
		if (player.is_underwater())
			player.get_phobj().flydir = 0;

		if (player.is_attacking())
			return;

		if (player.is_underwater())
		{
			// The arrows steer the swim: up glides, down drops, neither sinks.
			PhysicsObject &phobj = player.get_phobj();

			if (player.is_key_down(KeyAction::Id::LEFT))
				phobj.walkdir = -1;
			else if (player.is_key_down(KeyAction::Id::RIGHT))
				phobj.walkdir = 1;

			if (player.is_key_down(KeyAction::Id::UP))
				phobj.flydir = -1;
			else if (player.is_key_down(KeyAction::Id::DOWN))
				phobj.flydir = 1;

			return;
		}

		if (player.is_key_down(KeyAction::Id::LEFT))
			player.get_phobj().hforce = -player.get_flyforce();
		else if (player.is_key_down(KeyAction::Id::RIGHT))
			player.get_phobj().hforce = player.get_flyforce();

		if (player.is_key_down(KeyAction::Id::UP))
			player.get_phobj().vforce = -player.get_flyforce();
		else if (player.is_key_down(KeyAction::Id::DOWN))
			player.get_phobj().vforce = player.get_flyforce();
	}

	void PlayerFlyState::update_state(Player &player) const
	{
		// Out of a swimArea's water, the player falls or stands.
		if (!player.is_underwater())
		{
			player.set_state(player.get_phobj().onground ? ground_state(player) : Char::State::FALL);
			return;
		}

		if (player.get_phobj().onground)
		{
			Char::State state;

			if (player.is_key_down(KeyAction::Id::LEFT))
			{
				state = Char::State::WALK;
				player.set_direction(false);
			} else if (player.is_key_down(KeyAction::Id::RIGHT))
			{
				state = Char::State::WALK;
				player.set_direction(true);
			} else if (player.is_key_down(KeyAction::Id::DOWN))
			{
				state = Char::State::PRONE;
			} else
			{
				state = Char::State::STAND;
			}

			player.set_state(state);
		}
	}

	// Climbing
	void PlayerClimbState::initialize(Player &player) const
	{
		player.get_phobj().type = PhysicsObject::Type::FIXATED;
	}

	void PlayerClimbState::send_action(Player &player, KeyAction::Id ka, bool down) const
	{
		if (down)
		{
			switch (ka)
			{
				case KeyAction::Id::JUMP:
					if (player.is_key_down(KeyAction::Id::LEFT))
					{
						play_jumpsound();
						player.set_direction(false);
						player.get_phobj().hspeed = -ROPEJUMPH * Physics::walk_speed(player.get_walkspeed());
						player.get_phobj().vspeed = -(player.is_underwater() ? ROPEJUMPSWIM : ROPEJUMPV) * player.get_jumpforce();
						cancel_ladder(player);
					} else if (player.is_key_down(KeyAction::Id::RIGHT))
					{
						play_jumpsound();
						player.set_direction(true);
						player.get_phobj().hspeed = ROPEJUMPH * Physics::walk_speed(player.get_walkspeed());
						player.get_phobj().vspeed = -(player.is_underwater() ? ROPEJUMPSWIM : ROPEJUMPV) * player.get_jumpforce();
						cancel_ladder(player);
					}

					break;
			}
		}
	}

	void PlayerClimbState::update(Player &player) const
	{
		// Up and Down together cancel out.
		int8_t dir = player.is_key_down(KeyAction::Id::DOWN) - player.is_key_down(KeyAction::Id::UP);
		player.get_phobj().vspeed = dir * CLIMBSPEED;
	}

	void PlayerClimbState::update_state(Player &player) const
	{
		auto ladder = player.get_ladder();

		if (!ladder)
			return;

		// Past the top, the player stops, or steps 5 px above it if the
		// ladder leads up to a platform. Past the bottom, they let go 1 px
		// below it.
		PhysicsObject &phobj = player.get_phobj();
		double y = phobj.crnt_y();
		int16_t top = ladder->get_top();
		int16_t bottom = ladder->get_bottom();

		if (phobj.vspeed < 0.0 && y < top)
		{
			if (ladder->leads_up())
			{
				phobj.set_y(top - 5);
				cancel_ladder(player);
			} else
			{
				phobj.set_y(top);
			}
		} else if (phobj.vspeed > 0.0 && y > bottom)
		{
			phobj.set_y(bottom + 1);
			cancel_ladder(player);
		}
	}

	void PlayerClimbState::cancel_ladder(Player &player) const
	{
		player.set_state(Char::State::FALL);
		player.set_ladder(nullptr);
	}
}