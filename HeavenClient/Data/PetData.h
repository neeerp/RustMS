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
#pragma once

#include "../Character/Pets.h"
#include "../Template/Cache.h"

#include <string>
#include <vector>

#include <cstdint>

namespace ms
{
	// What a kind of pet knows, from its Item.nx image and String.nx's
	// PetDialog.img (CPetTemplate, loaded by FUN_0070af01).
	class PetData : public Cache<PetData>
	{
	public:
		// The kind's name (String.nx Pet.img), which a pet keeps until renamed.
		const std::string &get_kind() const;

		// The style of its chat balloon when called by name (info/chatBalloon).
		int8_t get_balloon() const;

		// Its commands, by the index sent for them.
		const std::vector<pets::Command> &get_commands() const;

		// How it answers food by level (food/N), with no words.
		const std::vector<pets::Command> &get_food() const;

		// What it may do when left standing (randAction).
		const std::vector<pets::Reaction> &get_idle() const;

		// Its own animations, actions 9 on.
		const std::vector<std::string> &get_customs() const;

	private:
		// Allow the cache to use the constructor
		friend Cache<PetData>;

		PetData(int32_t itemid);

		std::string kind;
		int8_t balloon;
		std::vector<pets::Command> commands;
		std::vector<pets::Command> food;
		std::vector<pets::Reaction> idle;
		std::vector<std::string> customs;
	};
}
