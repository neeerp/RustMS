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
#include "BuffParser.h"

#include "../Character/Diseases.h"

namespace ms
{
	namespace
	{
		// Bits 0..63 are the second long, 64..127 the first; within each,
		// the upper dword comes first.
		uint64_t mask(uint8_t bit)
		{
			uint8_t shift = bit % 64;

			return uint64_t(1) << (shift < 32 ? shift + 32 : shift - 32);
		}

		// The two-state stats (bits 82..88): each has a value, a reason and
		// when it was last updated, some with more (vtable +0x18 of each).
		constexpr uint8_t TWO_STATE = 82;
		constexpr size_t TWO_STATE_SIZE[7] = { 15, 15, 15, 13, 20, 17, 15 };

		enum Data
		{
			NOTHING,
			BYTE,
			SHORT,
			INT,
			SHORT_INT
		};

		// What another character's client reads for each stat it shows,
		// in its order.
		constexpr std::pair<uint8_t, Data> SHOWN[] = {
			{ 7, BYTE }, { 21, BYTE }, { 22, INT }, { 17, INT }, { 20, INT }, { 19, INT }, { 30, INT },
			{ 31, INT }, { 18, SHORT_INT }, { 26, NOTHING }, { 10, NOTHING }, { 16, NOTHING }, { 33, SHORT },
			{ 49, SHORT }, { 39, INT }, { 40, INT }, { 46, INT }, { 50, INT }, { 62, INT }, { 51, INT },
			{ 53, INT }, { 54, INT }, { 55, INT }, { 56, INT }, { 59, NOTHING }, { 60, NOTHING },
			{ 66, NOTHING }, { 73, INT }, { 75, INT }, { 76, INT }, { 77, INT }
		};

		// GIVE_BUFF reads in ascending order, except for these.
		constexpr uint8_t GIVEN[] = {
			0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
			21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 49, 34, 35, 36, 37, 38, 39,
			40, 41, 42, 43, 44, 45, 46, 47, 50, 62, 51, 48, 52, 53, 54, 55, 56, 59, 60, 61,
			63, 64, 65, 66, 67, 68, 69, 70, 71, 72, 73, 74, 57, 58, 75, 76, 77, 78, 79, 80, 81
		};
	}

	bool BuffParser::Flags::has(uint8_t bit) const
	{
		return ((bit < 64 ? second : first) & mask(bit)) != 0;
	}

	void BuffParser::Flags::set(uint8_t bit)
	{
		(bit < 64 ? second : first) |= mask(bit);
	}

	BuffParser::Flags BuffParser::read_flags(InPacket& recv)
	{
		uint64_t first = recv.read_long();
		uint64_t second = recv.read_long();

		return { first, second };
	}

	std::vector<uint8_t> BuffParser::given(const Flags& flags)
	{
		std::vector<uint8_t> bits;

		for (uint8_t bit : GIVEN)
			if (flags.has(bit))
				bits.push_back(bit);

		return bits;
	}

	std::vector<Buffstat::Id> BuffParser::stats(const Flags& flags)
	{
		std::vector<Buffstat::Id> ids;

		for (uint8_t bit = 0; bit < 128; bit++)
			if (flags.has(bit) && Buffstat::by_bit(bit) != Buffstat::NONE)
				ids.push_back(Buffstat::by_bit(bit));

		// Undead ends with Puppet's bit.
		if (flags.has(46))
			ids.push_back(Buffstat::UNDEAD);

		return ids;
	}

	BuffParser::TwoState BuffParser::read_two_state(InPacket& recv, const Flags& flags)
	{
		TwoState state{ 0, 0, { 0, 0, 0, 0 }, { 0, 0 } };

		for (uint8_t slot = 0; slot < 7; slot++)
		{
			if (!flags.has(TWO_STATE + slot))
				continue;

			if (TWO_STATE + slot == ENERGY)
			{
				state.energy = recv.read_int();
				state.energy_skill = recv.read_int();
				recv.skip(TWO_STATE_SIZE[slot] - 8);
			}
			else if (TWO_STATE + slot == DASH_SPEED || TWO_STATE + slot == DASH_JUMP)
			{
				int32_t value = recv.read_int();
				(TWO_STATE + slot == DASH_SPEED ? state.dash.speed : state.dash.jump) = value;
				state.dash.skill = recv.read_int();
				recv.skip(5); // the sign and the delta
				state.dash.seconds = recv.read_short();
			}
			else if (TWO_STATE + slot == RIDING)
			{
				state.ride.vehicle = recv.read_int();
				state.ride.skill = recv.read_int();
				recv.skip(TWO_STATE_SIZE[slot] - 8);
			}
			else
			{
				recv.skip(TWO_STATE_SIZE[slot]);
			}
		}

		return state;
	}

	std::vector<std::pair<Buffstat::Id, int32_t>> BuffParser::parse_shown(InPacket& recv, const Flags& flags)
	{
		std::vector<std::pair<Buffstat::Id, int32_t>> shown;

		for (auto [bit, data] : SHOWN)
		{
			if (!flags.has(bit))
				continue;

			int32_t value = 0;

			switch (data)
			{
			case NOTHING:
				break;
			case BYTE:
				value = static_cast<uint8_t>(recv.read_byte());
				break;
			case SHORT:
				value = recv.read_short();
				break;
			case INT:
				value = recv.read_int();
				break;
			case SHORT_INT:
				// Poison's n, then its reason, the mob skill whose
				// `affected` the character shows.
				recv.skip_short();
				value = recv.read_int();
				break;
			}

			Buffstat::Id stat = disease::given(Buffstat::by_bit(bit), value);

			if (stat != Buffstat::NONE)
				shown.emplace_back(stat, value);
		}

		recv.skip(2); // defense attack and state

		// A character shows its energy, its Dash, and the mount it's on.
		TwoState state = read_two_state(recv, flags);

		if (state.energy != 0)
			shown.emplace_back(Buffstat::ENERGY_CHARGE, state.energy);

		if (state.dash.speed != 0)
			shown.emplace_back(Buffstat::DASH_SPEED, state.dash.speed);

		if (state.ride.vehicle != 0)
			shown.emplace_back(Buffstat::MONSTER_RIDING, state.ride.vehicle);

		return shown;
	}
}
