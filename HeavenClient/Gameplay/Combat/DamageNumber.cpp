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
#include "DamageNumber.h"

#include "DamageLayout.h"

#include "../../Constants.h"
#include "../../Util/Randomizer.h"

#include <cmath>
#include <string>

#include <nlnx/nx.hpp>
#include <nlnx/node.hpp>

namespace ms
{
	DamageNumber::DamageNumber(Type type, int32_t damage, int16_t l) : lift(l)
	{
		rise.set(0.0f);
		opacity.set(1.0f);

		if (damage <= 0)
		{
			const Texture& miss = misses[type];
			Point<int16_t> origin = miss.get_origin();
			int16_t left = damage_layout::left(miss.width());

			glyphs.push_back({ &miss, { static_cast<int16_t>(left + origin.x()),
				static_cast<int16_t>(origin.y() - miss.height()) } });

			return;
		}

		std::string number = std::to_string(damage);
		std::vector<const Texture*> textures;
		std::vector<damage_layout::Digit> sizes;
		std::vector<int16_t> drops;
		Randomizer random;

		for (size_t i = 0; i < number.size(); i++)
		{
			const Texture& digit = digits[type][i == 0 ? 0 : 1][number[i] - '0'];

			textures.push_back(&digit);
			sizes.push_back({ digit.width(), digit.height(), digit.get_origin().x() });
			drops.push_back(random.next_int<int16_t>(2));
		}

		bool critical = type == Type::CRITICAL;
		std::vector<damage_layout::Placed> placed;
		int16_t width = damage_layout::place(sizes, drops, critical, placed);
		Point<int16_t> topleft(damage_layout::left(width), -damage_layout::TOP);

		if (critical)
			glyphs.push_back({ &critical_effect, topleft + critical_effect.get_origin() });

		for (size_t i = 0; i < textures.size(); i++)
		{
			Point<int16_t> corner(placed[i].left, placed[i].top);
			glyphs.push_back({ textures[i], topleft + corner + textures[i]->get_origin() });
		}
	}

	DamageNumber::DamageNumber()
	{}

	void DamageNumber::draw(double viewx, double viewy, float alpha) const
	{
		Point<int16_t> position(
			static_cast<int16_t>(std::round(anchor.x() + viewx)),
			static_cast<int16_t>(std::round(anchor.y() + viewy - rise.get(alpha)))
		);
		float interopc = opacity.get(alpha);

		for (const Glyph& glyph : glyphs)
			glyph.texture->draw({ position + glyph.offset, interopc });
	}

	void DamageNumber::set_anchor(Point<int16_t> a)
	{
		anchor = a - Point<int16_t>(0, lift);
	}

	bool DamageNumber::update()
	{
		elapsed += Constants::TIMESTEP;
		rise = damage_layout::rise(elapsed);
		opacity = damage_layout::opacity(elapsed);

		return elapsed >= damage_layout::DURATION;
	}

	void DamageNumber::init()
	{
		nl::node basic = nl::nx::effect["BasicEff.img"];
		const char* sets[NUM_TYPES] = { "NoRed", "NoCri", "NoViolet" };

		for (size_t type = 0; type < NUM_TYPES; type++)
		{
			std::string name = sets[type];
			nl::node first = basic[name + "1"];
			nl::node rest = basic[name + "0"];

			for (size_t digit = 0; digit < 10; digit++)
			{
				digits[type][0][digit] = first[std::to_string(digit)];
				digits[type][1][digit] = rest[std::to_string(digit)];
			}
		}

		// v83 shows a critical miss with the normal set.
		misses[Type::NORMAL] = basic["NoRed0"]["Miss"];
		misses[Type::CRITICAL] = basic["NoRed0"]["Miss"];
		misses[Type::TOPLAYER] = basic["NoViolet0"]["Miss"];
		critical_effect = basic["NoCri1"]["effect"];
	}

	Texture DamageNumber::digits[NUM_TYPES][2][10];
	Texture DamageNumber::misses[NUM_TYPES];
	Texture DamageNumber::critical_effect;
}
