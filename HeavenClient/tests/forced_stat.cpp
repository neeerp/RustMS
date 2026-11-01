#include "../Character/ForcedStat.h"

#include <cassert>
#include <cstddef>
#include <vector>

using namespace ms;
namespace fs = forced_stat;

namespace
{
	// The Burning Forests' packet as the server writes it, after the opcode.
	const std::vector<uint8_t> BURNING_FOREST = {
		0xE7, 3, 0xE7, 3, 0xE7, 3, 0xE7, 3, 0xFF, 0, 0xE7, 3, 0xE7, 3, 0x78, 0x8C
	};

	void test_decode()
	{
		size_t at = 0;
		auto read_short = [&]() {
			int16_t value = static_cast<int16_t>(BURNING_FOREST[at] | BURNING_FOREST[at + 1] << 8);
			at += 2;
			return value;
		};
		auto read_byte = [&]() { return static_cast<int8_t>(BURNING_FOREST[at++]); };

		fs::Values values = fs::decode(0x0F1F, read_short, read_byte);

		assert(at == BURNING_FOREST.size());
		assert(values[fs::STR] == 999 && values[fs::DEX] == 999 && values[fs::INT] == 999);
		assert(values[fs::LUK] == 999 && values[fs::PAD] == 255);
		assert(values[fs::PDD] == 0 && values[fs::MAD] == 0 && values[fs::MDD] == 0);
		assert(values[fs::ACC] == 999 && values[fs::EVA] == 999);
		// One-byte stats are unsigned: jump 140 is not -116.
		assert(values[fs::SPEED] == 120 && values[fs::JUMP] == 140 && values[fs::SPEED_MAX] == 0);
		assert(fs::any(values) && !fs::any(fs::Values{}));
	}

	void test_basic()
	{
		// A pinned STR replaces what is worn; Maple Warrior's 10% counts the player's own 12 only.
		assert(fs::basic(40, 12, 0.1f, 999) == 1000);
		// Unpinned, the percent counts the whole total.
		assert(fs::basic(40, 12, 0.1f, 0) == 44);
	}

	void test_secondary()
	{
		fs::Values values{};
		values[fs::PAD] = 255;
		values[fs::SPEED] = 120;
		values[fs::JUMP] = 140;

		assert(fs::secondary(fs::PAD, 17, values) == 255);
		// An unpinned one keeps its total.
		assert(fs::secondary(fs::PDD, 30, values) == 30);
		assert(fs::secondary(fs::SPEED, 100, values) == 120);
		// Jump stays within 123 whatever is pinned.
		assert(fs::secondary(fs::JUMP, 100, values) == 123);

		// A SpeedMax lets the speed past 140.
		values[fs::SPEED] = 180;
		assert(fs::secondary(fs::SPEED, 100, values) == 140);
		values[fs::SPEED_MAX] = 200;
		assert(fs::secondary(fs::SPEED, 100, values) == 180);
	}
}

int main()
{
	test_decode();
	test_basic();
	test_secondary();
}
