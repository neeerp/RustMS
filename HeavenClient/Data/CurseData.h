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

#include <string>

namespace ms
{
	// v83's chat filter (FUN_007a06f3) with the words of Etc/Curse.img.
	namespace CurseData
	{
		// The message with each curse word starred out.
		std::string filter(const std::string& message);
		// Whether a word of ForbiddenName.img or, up to a comma, of
		// Curse.img is anywhere in a name, ignoring case (FUN_0079fc74).
		bool in_name(const std::string& name);

		// Whether the filter takes the text at all: under 0x3ff bytes, with
		// no control character but a line break (FUN_007a04c0). Only such
		// text is refused with "Cursing is not allowed."; a curse word is
		// starred out and the text goes on.
		inline bool valid(const std::string& text)
		{
			if (text.size() >= 0x3ff)
				return false;

			for (char c : text)
				if (static_cast<unsigned char>(c) < 0x20 && c != '\r' && c != '\n')
					return false;

			return true;
		}
	}
}
