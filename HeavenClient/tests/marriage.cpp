#include "../Character/Marriage.h"
#include "../Net/InPacket.h"
#include <cstring>
#include <iostream>
#include <stdexcept>
#include <string>
#include <vector>

using namespace ms;

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

	// A name in its 13 bytes.
	Writer& padded(const std::string& name)
	{
		bytes.insert(bytes.end(), name.begin(), name.end());
		bytes.insert(bytes.end(), 13 - name.size(), 0);
		return *this;
	}

	InPacket packet() const
	{
		return InPacket(bytes.data(), bytes.size());
	}
};

void reads_the_couples_record()
{
	Writer writer;
	writer.put<int32_t>(7).put<int32_t>(100).put<int32_t>(200).put<int16_t>(1);
	writer.put<int32_t>(2240000).put<int32_t>(2240001).padded("Groom").padded("Bridetwelvech");
	require(writer.bytes.size() == 0x30, "the record is 0x30 bytes");

	InPacket recv = writer.packet();
	MarriageRecord record = MarriageRecord::read(recv);
	require(!recv.available(), "reads all of it");
	require(record.id == 7 && record.groom_id == 100 && record.bride_id == 200, "the ids");
	require(record.status == marriage::ENGAGED && !record.married(), "engaged");
	require(record.groom_item == 2240000 && record.bride_item == 2240001, "the boxes");
	require(record.groom_name == "Groom" && record.bride_name == "Bridetwelvech", "the names without padding");
	require(record.partner(false) == "Bridetwelvech" && record.partner(true) == "Groom", "each one's partner, by own gender");

	record.status = marriage::MARRIED;
	require(record.married(), "married");
}

void ring_boxes_and_result_texts()
{
	require(marriage::is_ring_box(2240000) && marriage::is_ring_box(2240003), "the boxes");
	require(!marriage::is_ring_box(2230000) && !marriage::is_ring_box(4031357), "not the boxes");

	require(std::string(marriage::result_text(marriage::RESULT_ENGAGED)) == "You are now engaged.", "engaged");
	require(std::string(marriage::result_text(18)) == "You have entered the wrong character name.", "wrong name");
	require(std::string(marriage::result_text(30)) == "She has politely declined your engagement request.", "declined");

	for (int8_t mode = 18; mode <= 30; mode++)
		require(marriage::result_text(mode) != nullptr, "every refusal has its text");

	require(!marriage::result_text(17) && !marriage::result_text(31), "no text around them");
	require(std::string(marriage::result_text(marriage::RESULT_BROKEN)) == "Your engagement has been broken.", "broken");
	require(std::string(marriage::result_text(marriage::RESULT_DIVORCED)) == "You are no longer married.", "divorced");
	require(marriage::result_text(32) != nullptr, "reserved");

	require(marriage::is_token(4031357) && marriage::is_token(4031364), "the tokens");
	require(!marriage::is_token(4031356) && !marriage::is_token(4031365), "not the tokens");
	require(std::string(marriage::break_question(false)).rfind("Dropping this item", 0) == 0, "engaged question");
	require(std::string(marriage::break_question(true)).rfind("You will not", 0) == 0, "married question");
	require(!marriage::result_text(marriage::RESULT_NOTICE), "the server's own text");
}

void tooltip_lines()
{
	MarriageRecord record;
	record.groom_name = "Groom";
	record.bride_name = "Bride";
	std::optional<MarriageRecord> couple = record;

	require(marriage::tooltip_line(4031357, couple, false) == "YOU ARE ENGAGED WITH : Bride", "a token, the groom");
	require(marriage::tooltip_line(4031364, couple, true) == "YOU ARE ENGAGED WITH : Groom", "a token, the bride");
	require(marriage::tooltip_line(1112803, couple, false) == "YOU ARE MARRIED WITH : Bride", "a wedding ring");
	require(marriage::tooltip_line(1112809, couple, true) == "YOU ARE MARRIED WITH : Groom", "the last ring");
	require(marriage::tooltip_line(4031357, std::nullopt, false).empty(), "no record, no line");
	require(marriage::tooltip_line(1112804, couple, false).empty(), "not a wedding ring");
	require(marriage::tooltip_line(2240000, couple, false).empty(), "a ring box has none");
}

void partner_spots()
{
	PartnerSpot none;
	require(!none.is(0), "id 0 is no one");
	require(!none.marks({ 0 }, 100000000), "map 0 is nowhere");

	PartnerSpot spot;
	spot.mapid = 102000000;
	spot.id = 4;
	require(spot.is(4) && !spot.is(5), "only the partner gets the dot");
	require(spot.marks({ 102000000, 102000100 }, 100000000), "the partner's spot");
	require(!spot.marks({ 100000000 }, 100000000), "another spot");
	require(!spot.marks({ 102000000, 102000100 }, 102000100), "the player's own spot wins");
}

int main()
{
	try
	{
		reads_the_couples_record();
		ring_boxes_and_result_texts();
		tooltip_lines();
		partner_spots();
	}
	catch (const std::exception& error)
	{
		std::cerr << "marriage: " << error.what() << "\n";
		return 1;
	}

	std::cout << "marriage: all passed\n";
	return 0;
}
