#include "../Net/Handlers/Helpers/MovementParser.h"
#include <cstring>
#include <iostream>
#include <stdexcept>
#include <vector>

void require(bool condition, const char* message)
{
	if (!condition) throw std::runtime_error(message);
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

	Writer& end(uint8_t stance, int16_t duration)
	{
		return put(stance).put(duration);
	}
};

int main()
{
	try
	{
		using namespace ms;

		Writer w;
		w.put<uint8_t>(7);
		// Walk: position, velocity, foothold.
		w.put<uint8_t>(0).put<int16_t>(100).put<int16_t>(-20).put<int16_t>(125).put<int16_t>(0).put<int16_t>(7).end(2, 90);
		// Jump: velocity only, at the walk's position.
		w.put<uint8_t>(1).put<int16_t>(125).put<int16_t>(-555).end(6, 300);
		// Type 10: one byte and nothing else.
		w.put<uint8_t>(10).put<uint8_t>(3);
		// Down jump: foothold 9, fallen from 8.
		w.put<uint8_t>(15).put<int16_t>(110).put<int16_t>(40).put<int16_t>(0).put<int16_t>(0).put<int16_t>(9).put<int16_t>(8).end(7, 120);
		// Teleport: position and foothold.
		w.put<uint8_t>(4).put<int16_t>(300).put<int16_t>(40).put<int16_t>(11).end(2, 0);
		// Velocity and the foothold fallen from.
		w.put<uint8_t>(14).put<int16_t>(-60).put<int16_t>(0).put<int16_t>(11).end(3, 40);
		// A type it doesn't know: stance and duration only.
		w.put<uint8_t>(21).end(5, 16);
		w.put<int8_t>(42);

		InPacket recv(w.bytes.data(), w.bytes.size());
		auto moves = MovementParser::parse_movements(recv, { -5, -6 });

		require(moves.size() == 7, "every element is read");
		require(recv.available() && recv.read_byte() == 42 && !recv.available(), "the path ends where it should");
		require(moves[0].type == Movement::ABSOLUTE && moves[0].xpos == 100 && moves[0].ypos == -20 && moves[0].vx == 125 && moves[0].fh == 7, "a walk has a position, velocity and foothold");
		require(moves[0].newstate == 2 && moves[0].duration == 90, "elements end with a stance and duration");
		require(moves[1].type == Movement::RELATIVE && moves[1].xpos == 100 && moves[1].ypos == -20 && moves[1].vy == -555, "a jump keeps the position before it");
		require(moves[2].type == Movement::NONE && moves[2].newstate == 6 && moves[2].xpos == 100, "type 10 keeps the stance before it");
		require(moves[3].type == Movement::JUMPDOWN && moves[3].fh == 9 && moves[3].newstate == 7, "a down jump lands on its first foothold");
		require(moves[4].type == Movement::ABSOLUTE && moves[4].xpos == 300 && moves[4].fh == 11, "a teleport has a position and foothold");
		require(moves[5].type == Movement::RELATIVE && moves[5].xpos == 300 && moves[5].vx == -60, "type 14 is a velocity");
		require(moves[6].type == Movement::NONE && moves[6].newstate == 5 && moves[6].duration == 16, "an unknown type still has a stance");

		Writer empty;
		empty.put<uint8_t>(0);
		InPacket none(empty.bytes.data(), empty.bytes.size());
		require(MovementParser::parse_movements(none, {}).empty(), "a path may have no elements");

		require(Movement::velocity(1.0) == 125 && Movement::velocity(-4.44) == -555, "speeds are per second");

		std::cout << "Movement path passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
