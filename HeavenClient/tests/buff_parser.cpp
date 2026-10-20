#include "../Net/Handlers/Helpers/BuffParser.h"
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

	Writer& flags(const ms::BuffParser::Flags& flags)
	{
		return put(flags.first).put(flags.second);
	}

	Writer& zeros(size_t count)
	{
		bytes.insert(bytes.end(), count, 0);
		return *this;
	}
};

int main()
{
	try
	{
		using namespace ms;

		BuffParser::Flags speed{ 0, 0 };
		speed.set(7);
		require(speed.second == 0x8000000000ULL && speed.first == 0, "Speed is bit 7, in the second long");

		BuffParser::Flags flags{ 0, 0 };
		flags.set(34);
		flags.set(82);
		require(flags.second == 0x4 && flags.first == 0x4000000000000ULL,
				"Recovery is bit 34 and energy charge bit 82, in the first long");

		BuffParser::Flags given{ 0, 0 };
		for (uint8_t bit : { 14, 13, 34, 49, 58, 74, 82 })
			given.set(bit);
		require(BuffParser::given(given) == std::vector<uint8_t>({ 13, 14, 49, 34, 74, 58 }),
				"GIVE_BUFF reads in ascending order with v83's exceptions, and not the two-state stats");

		BuffParser::Flags hyperbody{ 0, 0 };
		hyperbody.set(13);
		hyperbody.set(14);
		require(BuffParser::stats(hyperbody) == std::vector<Buffstat::Id>({ Buffstat::HYPERBODYHP, Buffstat::HYPERBODYMP }),
				"each set bit names its stat");
		require(Buffstat::by_bit(48) == Buffstat::NONE, "stats HeavenClient doesn't keep are NONE");

		// SPAWN_PLAYER: Speed and Dark Sight, then all seven two-state stats.
		BuffParser::Flags spawn{ 0, 0 };
		for (uint8_t bit : { 7, 10, 82, 83, 84, 85, 86, 87, 88 })
			spawn.set(bit);
		Writer w;
		w.flags(spawn).put<uint8_t>(140).zeros(2).zeros(110).put<int16_t>(412);
		InPacket recv(w.bytes.data(), w.bytes.size());
		auto shown = BuffParser::parse_shown(recv, BuffParser::read_flags(recv));
		require(shown.size() == 2 && shown[0].first == Buffstat::SPEED && shown[0].second == 140 &&
				shown[1].first == Buffstat::DARKSIGHT, "a spawned character shows its buffs");
		require(recv.read_short() == 412 && !recv.available(), "the two-state stats take 110 bytes");

		// GIVE_FOREIGN_BUFF: the data comes in the client's fixed order.
		BuffParser::Flags remote{ 0, 0 };
		for (uint8_t bit : { 18, 21, 22, 26 })
			remote.set(bit);
		Writer c;
		c.flags(remote).put<uint8_t>(3).put<int32_t>(1211004).put<int16_t>(5).put<int32_t>(125 | 3 << 16).zeros(2).put<int16_t>(0);
		InPacket buff(c.bytes.data(), c.bytes.size());
		shown = BuffParser::parse_shown(buff, BuffParser::read_flags(buff));
		require(shown.size() == 4 && shown[0].first == Buffstat::COMBO && shown[0].second == 3 &&
				shown[1].first == Buffstat::WK_CHARGE && shown[1].second == 1211004 &&
				shown[2].first == Buffstat::POISON && shown[2].second == (125 | 3 << 16) &&
				shown[3].first == Buffstat::SHADOWPARTNER, "combo, charge, poison (by its reason) and shadow partner");
		require(buff.read_short() == 0 && !buff.available(), "the delay follows");

		// Undead shares Puppet's bit 46, told apart by its mob skill; Stop
		// Potion, Stop Motion and Fear each read an int.
		BuffParser::Flags cursed{ 0, 0 };
		for (uint8_t bit : { 46, 75, 76, 77 })
			cursed.set(bit);
		Writer d;
		d.flags(cursed).put<int32_t>(133 | 1 << 16).put<int32_t>(134 | 1 << 16).put<int32_t>(135 | 1 << 16)
			.put<int32_t>(136 | 1 << 16).zeros(2).put<int16_t>(0);
		InPacket curse(d.bytes.data(), d.bytes.size());
		shown = BuffParser::parse_shown(curse, BuffParser::read_flags(curse));
		require(shown.size() == 4 && shown[0].first == Buffstat::UNDEAD && shown[1].first == Buffstat::STOP_POTION &&
				shown[2].first == Buffstat::STOP_MOTION && shown[3].first == Buffstat::FEAR,
				"Undead, Stop Potion, Stop Motion and Fear");
		require(curse.read_short() == 0 && !curse.available(), "and the delay follows");

		// GIVE_FOREIGN_BUFF: another character gets on a mount.
		BuffParser::Flags riding{ 0, 0 };
		riding.set(BuffParser::RIDING);
		Writer e;
		e.flags(riding).zeros(2).put<int32_t>(1902000).put<int32_t>(1004).zeros(5).put<int16_t>(0);
		InPacket ride(e.bytes.data(), e.bytes.size());
		shown = BuffParser::parse_shown(ride, BuffParser::read_flags(ride));
		require(shown.size() == 1 && shown[0].first == Buffstat::MONSTER_RIDING && shown[0].second == 1902000,
				"a character shows the mount it rides");
		require(ride.read_short() == 0 && !ride.available(), "riding's data takes 13 bytes");

		// GIVE_BUFF: riding's data comes after the two-state stats before it.
		riding.set(83);
		Writer g;
		g.zeros(15).put<int32_t>(1932000).put<int32_t>(5221006).zeros(5).put<int16_t>(0);
		InPacket own(g.bytes.data(), g.bytes.size());
		BuffParser::Ride got = BuffParser::read_two_state(own, riding);
		require(got.vehicle == 1932000 && got.skill == 5221006, "the mount and the skill riding it");
		require(own.read_short() == 0 && !own.available(), "and the delay follows");

		std::cout << "Buff parser passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
