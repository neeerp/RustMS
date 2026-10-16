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
#pragma once

#include <cstdint>

#include "SkillAction.h"
#include "SkillBullet.h"
#include "SkillHitEffect.h"
#include "SkillSound.h"
#include "SkillUseEffect.h"
#include "SpecialMove.h"

#include <memory>

namespace ms
{
	// What a skill held down shows: its prepare action, and the effects
	// while it's prepared, held and let go (0x75d58e).
	struct SkillPrepare
	{
		struct Effect
		{
			Animation animation;
			int8_t z = 0;
		};

		std::string action;
		// When the prepare effect turns into the keydown one.
		int32_t time = 0;
		Effect prepare;
		Effect keydown;
		Effect keydownend;
	};

	// The skill implementation of special move
	class Skill : public SpecialMove
	{
	public:
		Skill(int32_t skillid);

		void apply_useeffects(Char &user) const override;

		void apply_actions(Char &user, Attack::Type type) const override;

		void apply_stats(const Char &user, Attack &attack) const override;

		void apply_hiteffects(const AttackUser &user, Mob &target) const override;

		Animation get_bullet(const Char &user, int32_t bulletid) const override;

		Animation get_ball(size_t kind) const override;

		bool is_attack() const override;

		bool is_skill() const override;

		int32_t get_id() const override;

		const SkillPrepare &get_prepare() const;

		// Play the sound of using the skill, without its effect.
		void play_use() const;

		// Play the sound a held skill makes while it charges, until
		// stop_loop.
		void play_loop() const;

		void stop_loop() const;

		ForbidReason can_use(int32_t level, Weapon::Type weapon, const Job &job, uint16_t hp, uint16_t mp,
							 uint16_t bullets) const override;

	private:
		std::unique_ptr<SkillAction> action;
		std::unique_ptr<SkillBullet> bullet;
		std::unique_ptr<SkillSound> sound;
		std::unique_ptr<SkillUseEffect> useeffect;
		std::unique_ptr<SkillHitEffect> hiteffect;
		SkillPrepare prepare;

		int32_t skillid;
		bool overregular;
		bool projectile;
	};
}