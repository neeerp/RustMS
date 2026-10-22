#include "../Net/Handlers/Helpers/MobStatParser.h"
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

	Writer& status(int16_t value, int32_t reason, int16_t duration)
	{
		return put(value).put(reason).put(duration);
	}
};

int main()
{
	try
	{
		using namespace ms;

		// Doom (16) and Web (17) come after the immunities (18, 19), and
		// the counters' data after every status.
		BuffParser::Flags flags{ 0, 0 };
		for (uint8_t stat : { 6, 16, 17, 18, 29 })
			flags.set(stat);

		Writer w;
		w.status(-20, 2101003, 40)
			.status(1, 2311001, 2)
			.status(1, 2311005, 30)
			.status(1, 4111003, 16)
			.status(5, 1, 6)
			.put(int32_t(7))
			.put(int32_t(8))
			.put(int16_t(-1));

		InPacket recv(w.bytes.data(), w.bytes.size());
		std::vector<MobStatParser::Status> statuses = MobStatParser::parse(recv, flags);

		require(statuses.size() == 5, "Every plain status set is read");
		require(statuses[0].stat == MobStatParser::SPEED && statuses[0].value == -20 &&
				statuses[0].reason == 2101003 && statuses[0].duration == 40, "Slow's speed comes first");
		require(statuses[1].stat == MobStatParser::PIMMUNE, "The immunity comes before Doom");
		require(statuses[2].stat == MobStatParser::DOOM && statuses[2].reason == 2311005, "Then Doom");
		require(statuses[3].stat == MobStatParser::WEB && statuses[3].duration == 16, "Then Web");
		require(statuses[4].stat == MobStatParser::PCOUNTER, "And the counter");
		require(statuses[4].spread == 7 && statuses[4].chance == 8, "The counter keeps its spread and chance");
		require(recv.read_short() == -1, "The counter's two ints are read");

		// Its value give or take half its spread, when the chance comes up.
		MobStatParser::Status counter = { MobStatParser::PCOUNTER, 220, 145 | 5 << 16, 20, 98, 0 };
		require(MobStatParser::counter(counter, 99, 0) == 220 - 49, "Half the spread off");
		require(MobStatParser::counter(counter, 99, 97) == 220 + 48, "Up to half the spread on");
		counter.spread = 0;
		require(MobStatParser::counter(counter, 99, 97) == 220, "No spread, just the value");
		counter.chance = 30;
		require(MobStatParser::counter(counter, 129, 0) == 220, "Under the chance it hurts");
		require(MobStatParser::counter(counter, 130, 0) == 0, "At it or over, not");

		require(MobStatParser::stops(flags), "Doom stops the mob");

		BuffParser::Flags burned{ 0, 0 };
		burned.set(MobStatParser::BURNED);
		Writer b;
		b.put(int32_t(2)).put(int32_t(1)).put(int32_t(2)).put(int8_t(9));

		InPacket reset(b.bytes.data(), b.bytes.size());
		MobStatParser::skip_reset(reset, burned);
		require(reset.read_byte() == 9, "A reset reads past the burns' ids");
		require(!MobStatParser::stops(burned), "Burns don't stop the mob");

		// Hypnotize's Dazzle (28) is a plain status after the burns.
		BuffParser::Flags dazzle{ 0, 0 };
		dazzle.set(MobStatParser::DAZZLE);
		Writer d;
		d.status(1, 5221009, 21).put(int8_t(9));

		InPacket dazzled(d.bytes.data(), d.bytes.size());
		std::vector<MobStatParser::Status> hypnotized = MobStatParser::parse(dazzled, dazzle);
		require(hypnotized.size() == 1 && hypnotized[0].stat == 28 && hypnotized[0].reason == 5221009 &&
				hypnotized[0].duration == 21, "Dazzle is read as bit 28");
		require(dazzled.read_byte() == 9, "And nothing more");
		require(!MobStatParser::stops(dazzle), "Dazzle doesn't stop the mob");

		std::cout << "mob stat parser tests passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
