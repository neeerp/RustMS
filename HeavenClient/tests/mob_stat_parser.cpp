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
		require(recv.read_short() == -1, "The counter's two ints are read past");

		require(MobStatParser::stops(flags), "Doom stops the mob");

		BuffParser::Flags burned{ 0, 0 };
		burned.set(MobStatParser::BURNED);
		Writer b;
		b.put(int32_t(2)).put(int32_t(1)).put(int32_t(2)).put(int8_t(9));

		InPacket reset(b.bytes.data(), b.bytes.size());
		MobStatParser::skip_reset(reset, burned);
		require(reset.read_byte() == 9, "A reset reads past the burns' ids");
		require(!MobStatParser::stops(burned), "Burns don't stop the mob");

		std::cout << "mob stat parser tests passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
