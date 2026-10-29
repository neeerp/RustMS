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
#include <functional>
#include <optional>
#include <string>
#include <vector>

namespace ms
{
	// What the Cash Shop asks before it sends a gift, from the receiver's
	// data the server looked up (v83 46ec85, reports b331 and b347).
	namespace cash_gift
	{
		// The receiver's facts the checks read.
		struct Receiver
		{
			std::string name;
			int16_t job = 0;
			int16_t level = 1;
			int16_t fame = 0;
			// The first pet's serial is set and an item in the Cash tab has it.
			bool pet = false;
			// Some item, normal worn equipment or a dragon's has
			// tradeAvailable and no karma mark yet.
			bool karma_target = false;
			// The worn weapon (slot -11), 0 for none.
			int32_t weapon = 0;
			// Monster Riding (1004 or 10001004) is learned.
			bool riding = false;
		};

		// An equip's reqJob, reqLevel and reqPOP.
		struct Equip
		{
			int16_t job = 0;
			int16_t level = 0;
			int16_t fame = 0;
		};

		// What the checks look up: an equip's data, empty when there's none,
		// and whether a cash weapon has art for a weapon's type (as
		// cash_weapon::refusal's `covers`).
		struct Lookup
		{
			std::function<std::optional<Equip>(int32_t)> equip;
			std::function<bool(int32_t, int32_t)> covers;
		};

		// A package's member, or the item itself.
		struct Member
		{
			int32_t item_id;
			// Days the item lasts; 0 is forever.
			int32_t period = 0;
		};

		struct Question
		{
			// A Yes/No question; otherwise an OK box that ends the gift.
			bool ask;
			std::string text;
		};

		// StringPool 0x155f.
		constexpr const char* EXPIRES = "This contains an item that will disappear once \\nthe expiration date has passed. Do you want to give it as a gift?";
		// StringPool 0x22d.
		constexpr const char* UNKNOWN_ERROR = "Due to an unknown error,\\nthe request for Cash Shop has failed.";

		inline std::string with_name(const char* text, const std::string& name)
		{
			std::string out = text;
			out.replace(out.find("%s"), 2, name);

			return out;
		}

		// Whether the receiver can use the item (v83 46d5e2): the first
		// check that fails gives its question, and none gives nothing.
		inline std::optional<Question> use_check(int32_t item_id, const Receiver& receiver, const Lookup& lookup)
		{
			int32_t kind = item_id / 10000;

			if (kind == 519 || kind == 524)
			{
				if (!receiver.pet)
					return Question{ true, "This item cannot be used without a pet, and\\nthis item cannot be used on a deceased pet.\\nAre you sure you want to make the purchase?" };

				return std::nullopt;
			}

			if (kind == 552)
			{
				if (!receiver.karma_target)
					return Question{ true, with_name("%s does not have an item that \\nthe Scissors of Karma can be used on.\\nDo you still wish to give it as a gift?", receiver.name) };

				return std::nullopt;
			}

			if (item_id / 1000000 != 1)
				return std::nullopt;

			std::optional<Equip> data = lookup.equip(item_id);

			if (!data)
				return Question{ false, UNKNOWN_ERROR };

			int32_t job_class = (receiver.job % 1000) / 100;
			int32_t job_bit = job_class != 0 ? 1 << (job_class - 1) : 0;

			if ((data->job != 0 && (data->job & job_bit) == 0) || data->level > receiver.level || (data->fame != 0 && data->fame > receiver.fame))
				return Question{ true, with_name("Due to job limitations, %s can't\\nuse this item.\\nDo you still wish to give it as a gift?", receiver.name) };

			if (item_id / 100000 == 17 && receiver.weapon != 0 && !lookup.covers(item_id, receiver.weapon))
				return Question{ true, with_name("With the weapon that %s currently has equipped,\\nthis item can't be used.\\nDo you still wish to give it as a gift?", receiver.name) };

			if ((kind == 190 || kind == 191) && !receiver.riding)
				return Question{ true, "This is an item that cannot be used\\nif you do not have the Monster\\nRiding skill. Would you\\nstill like to purchase it?" };

			return std::nullopt;
		}

		// The questions before a gift, in order (v83 46ec85). Each is asked
		// once the one before it is answered yes; an OK box ends the gift.
		// A package asks at most one, for its first member that needs one.
		// An item asks whether it may expire, then whether it can be used.
		inline std::vector<Question> questions(bool package, const std::vector<Member>& members, const Receiver& receiver,
											   const Lookup& lookup)
		{
			std::vector<Question> asked;

			for (const Member& member : members)
			{
				if (member.period != 0)
					asked.push_back({ true, EXPIRES });

				if (!package || asked.empty())
				{
					if (auto question = use_check(member.item_id, receiver, lookup))
						asked.push_back(*question);
				}

				if (package && !asked.empty())
					break;
			}

			return asked;
		}
	}
}
