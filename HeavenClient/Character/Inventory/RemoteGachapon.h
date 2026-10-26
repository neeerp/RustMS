#pragma once

#include <array>
#include <cstdint>

namespace ms
{
	// The Remote Gachapon Ticket (5451xxx, v83 cash type 0x3b, report b196). Double-clicked in the Cash tab it opens
	// a window of ten towns; a town's button sends the ticket with the button's index and closes the window, and the
	// server answers as that town's machine.
	namespace remote_gachapon
	{
		constexpr int32_t TICKET = 5451000;

		// 0x48645b: class 545 is type 0x3b only for 5451xxx; Miu Miu (5450000) is type 0x25.
		constexpr bool is_ticket(int32_t item_id)
		{
			return item_id / 1000 == 5451;
		}

		struct Place
		{
			int16_t x;
			int16_t y;
		};

		// UIWindow.img/RemoteGachapon/Bt0..Bt9 from the window's top left (0x8111ac), in the order of the index
		// they send (0x811a64): Henesys, Ellinia, Perion, Kerning City, Sleepywood, Mushroom Shrine, the Showa Spa's
		// men's and women's, New Leaf City and Nautilus Harbor. The screen's order differs.
		constexpr std::array<Place, 10> TOWNS = {{
			{ 15, 113 }, { 128, 113 }, { 15, 134 }, { 128, 134 }, { 128, 155 },
			{ 15, 176 }, { 15, 197 }, { 128, 197 }, { 128, 176 }, { 15, 155 }
		}};

		// Basic.img/BtClose.
		constexpr Place CLOSE = { 227, 8 };
	}
}
