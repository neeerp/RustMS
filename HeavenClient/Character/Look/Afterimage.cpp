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
#include "Afterimage.h"

#include "../Util/Misc.h"

#include <nlnx/nx.hpp>

namespace ms
{
	Afterimage::Afterimage(int32_t skill_id, const std::string &name, const std::string &action,
						   const std::string &stance_name, int16_t level, uint8_t charge)
	{
		nl::node set = nl::nx::character["Afterimage"][name + ".img"];

		auto find = [&](const std::string &key)
		{
			nl::node found;

			if (skill_id > 0)
			{
				std::string strid = string_format::extend_id(skill_id, 7);
				found = nl::nx::skill[string_format::skill_img(skill_id)]["skill"][strid]["afterimage"][name][key];
			}

			if (!found && charge > 0)
				found = set["charge"][std::to_string(charge)][key];

			if (!found)
				found = set[level / 10][key];

			return found;
		};

		// Blast plays its own action and reaches as stabO2 (0x9513b6).
		nl::node src = find(skill_id == BLAST ? "stabO2" : action);
		action_frames = src && skill_id != BLAST;

		if (!src)
			src = find(stance_name);

		// Savage Blow's action has no afterimage, only this reach (0x414456).
		range = action == "savage" ? Rectangle<int16_t>(-88, -18, -62, -6) : Rectangle<int16_t>(src);
		firstframe = 0;
		displayed = false;

		for (nl::node sub : src)
		{
			uint8_t frame = string_conversion::or_default<uint8_t>(sub.name(), 255);

			if (frame < 255)
			{
				animation = sub;
				firstframe = frame;
			}
		}
	}

	Afterimage::Afterimage()
	{
		firstframe = 0;
		displayed = true;
		action_frames = false;
	}

	void Afterimage::draw(uint8_t stframe, const DrawArgument &args, float alpha) const
	{
		if (!displayed && stframe >= firstframe)
			animation.draw(args, alpha);
	}

	void Afterimage::update(uint8_t stframe, uint16_t timestep)
	{
		if (!displayed && stframe >= firstframe)
			displayed = animation.update(timestep);
	}

	uint8_t Afterimage::get_first_frame() const
	{
		return firstframe;
	}

	Rectangle<int16_t> Afterimage::get_range() const
	{
		return range;
	}

	bool Afterimage::counts_action_frames() const
	{
		return action_frames;
	}
}