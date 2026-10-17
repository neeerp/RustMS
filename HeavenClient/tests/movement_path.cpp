#include "../Gameplay/MovePath.h"
#include "../Net/Handlers/Helpers/MovementParser.h"
#include <cmath>
#include <cstring>
#include <iostream>
#include <stdexcept>
#include <vector>

void require(bool condition, const char* message)
{
	if (!condition) throw std::runtime_error(message);
}

ms::Movement at(int16_t x, int16_t y, int16_t vx, int16_t vy, int16_t fh, uint8_t stance = 2)
{
	return ms::Movement(ms::Movement::ABSOLUTE, 0, x, y, vx, vy, fh, stance, 0);
}

// Steps of 8 ms, as HC records them, moving `vx` px/s from `x` each step.
int16_t walk(ms::move_path::Recorder& path, int16_t x, int16_t vx, int16_t fh, int steps, int16_t vy = 0)
{
	for (int i = 0; i < steps; i++)
	{
		x += vx / 125;
		path.step(at(x, 0, vx, vy, fh), 8);
	}

	return x;
}

void recorder()
{
	using namespace ms::move_path;

	Recorder path;
	path.reset(at(0, 0, 0, 0, 7));

	int16_t x = walk(path, 0, 125, 7, 124);
	require(!path.flush(), "nothing goes out before a second");

	x = walk(path, x, 125, 7, 1);
	auto sent = path.flush();
	require(sent && sent->start == ms::Point<int16_t>(0, 0), "a second of walking goes out from where it began");
	require(sent->elements.size() == 1 && sent->elements[0].duration == 1000 && sent->elements[0].xpos == x,
			"a walk in one direction is one element");

	walk(path, x, 0, 7, 125);
	sent = path.flush();
	require(sent && sent->start == ms::Point<int16_t>(x, 0) && sent->elements.size() == 1,
			"the next path starts where the last one ended");
	walk(path, x, 0, 7, 125);
	require(!path.flush(), "standing still sends nothing");

	x = walk(path, x, 125, 7, 60);
	x = walk(path, x, -125, 7, 10);
	x = walk(path, x, 125, 0, 40, -500);
	x = walk(path, x, 125, 0, 40, 500);
	sent = path.flush();
	require(sent && sent->elements.size() == 2 && sent->elements[1].vx < 0,
			"turning around starts an element, and a jump waits");
	require(!path.flush(), "the jump stays for the next path");

	int16_t before = sent->elements[1].xpos;
	x = walk(path, x, 125, 7, 45);
	sent = path.flush();
	require(sent && sent->start == ms::Point<int16_t>(before, 0) && sent->elements.size() == 3,
			"the jump goes out once the player lands");
	require(sent->elements[0].fh == 0 && sent->elements[0].vy < 0 && sent->elements[1].vy > 0,
			"rising and falling are separate elements");

	path.add(ms::Movement(ms::Movement::ABSOLUTE, 4, x, 0, 0, 0, 7, 2, 5));
	walk(path, 200, 0, 7, 125);
	sent = path.flush();
	require(sent && sent->elements.size() == 2 && sent->elements[0].command == 4 &&
			sent->elements[0].duration == 0 && sent->elements[0].xpos == x && sent->elements[1].xpos == 200,
			"a teleport marks where it left from");

	walk(path, 200, 0, -3, 200);
	require(!path.flush(), "a climb waits for a foothold");
}

void player()
{
	using namespace ms::move_path;

	Player path({ 0, 0, 0, 0, 7, 2 });
	ms::Movement first = at(100, 0, 100, 0, 7, 3);
	first.duration = 1000;
	path.receive({ first });
	path.receive({});
	require(path.playing(), "a path plays at once");

	path.step(500);
	require(std::abs(path.get_state().x - 37.5) < 1e-9 && path.get_state().stance == 3,
			"it follows a curve between the two speeds");

	path.step(500);
	require(!path.playing() && path.get_state().x == 100 && path.get_state().vx == 100,
			"it ends where the path does");

	ms::Movement skip = at(0, 0, 0, 0, 0);
	skip.command = 10;
	ms::Movement walk = at(300, 0, 0, 0, 7, 4);
	walk.duration = 1200;
	path.receive({ skip, walk });
	path.step(100);
	// 110 ms into 1200, leaving 100 at 100 px/s for 300.
	require(std::abs(path.get_state().x - 113.8093287) < 1e-6, "a path of 1.1 seconds or more plays a tenth faster");

	walk.duration = 7000;
	walk.xpos = -50;
	path.receive({ walk });
	path.step(8);
	require(!path.playing() && path.get_state().x == -50, "one too far behind jumps to its end");
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

		recorder();
		player();

		std::cout << "Movement path passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
