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
		require(Buffstat::by_bit(29) == Buffstat::NONE, "stats HeavenClient doesn't keep are NONE");

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
		c.flags(remote).put<uint8_t>(3).put<int32_t>(1211004).put<int16_t>(5).put<int32_t>(2111003).zeros(2).put<int16_t>(0);
		InPacket buff(c.bytes.data(), c.bytes.size());
		shown = BuffParser::parse_shown(buff, BuffParser::read_flags(buff));
		require(shown.size() == 4 && shown[0].first == Buffstat::COMBO && shown[0].second == 3 &&
				shown[1].first == Buffstat::WK_CHARGE && shown[1].second == 1211004 &&
				shown[2].first == Buffstat::POISON && shown[2].second == 5 &&
				shown[3].first == Buffstat::SHADOWPARTNER, "combo, charge, poison and shadow partner");
		require(buff.read_short() == 0 && !buff.available(), "the delay follows");

		std::cout << "Buff parser passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
