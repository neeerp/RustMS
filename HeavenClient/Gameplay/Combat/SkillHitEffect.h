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

#include "../MapleMap/Mob.h"
#include "../../Template/BoolPair.h"

namespace ms
{
	// Interface for hit effects, animations applied to a mob for each hit.
	class SkillHitEffect
	{
	public:
		virtual ~SkillHitEffect()
		{}

		virtual void apply(const AttackUser &user, Mob &target) const = 0;

	protected:
		class Effect
		{
		public:
			Effect(nl::node src)
			{
				animation = src;
				pos = src["pos"];
				// As in the MapleRoyals client, above the mob unless the
				// node says otherwise (0x669773).
				z = src["z"].data_type() == nl::node::type::integer ? static_cast<int8_t>(src["z"]) : 1;
			}

			// Flipped when the attacker faces right.
			void apply(Mob &target, const AttackUser &user) const
			{
				target.show_effect(animation, pos, z, user.flip, user.range);
			}

		private:
			Animation animation;
			int8_t pos;
			int8_t z;
		};
	};

	// No animation
	class NoHitEffect : public SkillHitEffect
	{
	public:
		void apply(const AttackUser &, Mob &) const override
		{}
	};

	// A single animation
	class SingleHitEffect : public SkillHitEffect
	{
	public:
		SingleHitEffect(nl::node src);

		void apply(const AttackUser &user, Mob &target) const override;

	private:
		Effect effect;
	};

	// The animation changes depending on the weapon used
	class TwoHandedHitEffect : public SkillHitEffect
	{
	public:
		TwoHandedHitEffect(nl::node src);

		void apply(const AttackUser &user, Mob &target) const override;

	private:
		BoolPair<Effect> effects;
	};

	// The animation changes with the character level
	class ByLevelHitEffect : public SkillHitEffect
	{
	public:
		ByLevelHitEffect(nl::node src);

		void apply(const AttackUser &user, Mob &target) const override;

	private:
		std::map<uint16_t, Effect> effects;
	};

	// The animation changes with the character level and weapon used
	class ByLevelTwoHandedHitEffect : public SkillHitEffect
	{
	public:
		ByLevelTwoHandedHitEffect(nl::node src);

		void apply(const AttackUser &user, Mob &target) const override;

	private:
		std::map<uint16_t, BoolPair<Effect>> effects;
	};

	// The animation changes with the skill level
	class BySkillLevelHitEffect : public SkillHitEffect
	{
	public:
		BySkillLevelHitEffect(nl::node src);

		void apply(const AttackUser &user, Mob &target) const override;

	private:
		std::map<int32_t, Effect> effects;
	};

	// The animation is the one for the element of the user's charge, under
	// hit/0 (Charged Blow, Blast).
	class ByChargeHitEffect : public SkillHitEffect
	{
	public:
		ByChargeHitEffect(nl::node src);

		void apply(const AttackUser &user, Mob &target) const override;

	private:
		std::map<uint8_t, Effect> effects;
	};
}