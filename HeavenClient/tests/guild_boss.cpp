#include "../Character/GuildBoss.h"

#include <cstring>
#include <iostream>
#include <stdexcept>
#include <vector>

namespace
{
	void require(bool condition, const char* message)
	{
		if (!condition)
			throw std::runtime_error(message);
	}

	struct Writer
	{
		std::vector<int8_t> bytes;

		template <typename T>
		Writer& put(T value)
		{
			int8_t raw[sizeof(T)];
			std::memcpy(raw, &value, sizeof(T));
			bytes.insert(bytes.end(), raw, raw + sizeof(T));
			return *this;
		}

		ms::InPacket packet() const
		{
			return ms::InPacket(bytes.data(), bytes.size());
		}
	};
}

int main()
{
	try
	{
		using namespace ms;

		// Map 990000900's healer and pulley.
		guild_boss::Rules rules;
		rules.healer_x = 295;
		rules.y_min = -564;
		rules.pulley_x = -59;
		rules.pulley_y = -394;

		guild_boss::enter_field(guild_boss::FIELD_TYPE, rules);
		require(guild_boss::on_field(), "type 8 is Ergoth's room");
		require(guild_boss::state().healer_y == -564, "the chandelier starts at the top");
		require(guild_boss::state().pulley == 0, "the pulley starts free");

		// The pulley's box is left of and below its picture's origin.
		guild_boss::Rect box = rules.pulley_box();
		require(box.left == -245 && box.right == -119, "the box's sides");
		require(box.top == -304 && box.bottom == -210, "the box's top and bottom");
		require(box.intersects({ -130, -280, -20, -220 }), "a swing reaching in from the right");
		require(!box.intersects({ -119, -280, -20, -220 }), "one ending at its right edge misses");
		require(!box.intersects({ -300, -400, -200, -304 }), "one above it misses");

		Writer move;
		move.put<int16_t>(-420);
		InPacket moved = move.packet();
		require(guild_boss::read_healer_move(moved) == -420, "the chandelier's height");

		Writer lock;
		lock.put<uint8_t>(1);
		InPacket locked = lock.packet();
		require(guild_boss::read_pulley_state(locked) == 1, "the pulley's state");

		// Any other field forgets it, and the next room starts afresh.
		guild_boss::state().healer_y = -400;
		guild_boss::enter_field(0, rules);
		require(!guild_boss::on_field(), "a plain field");
		guild_boss::enter_field(guild_boss::FIELD_TYPE, rules);
		require(guild_boss::state().healer_y == -564, "back at the top");
	}
	catch (const std::exception& e)
	{
		std::cerr << "guild_boss: " << e.what() << '\n';
		return 1;
	}

	std::cout << "guild_boss: ok\n";
	return 0;
}
