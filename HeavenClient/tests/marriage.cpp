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
	require(record.partner(100) == "Bridetwelvech" && record.partner(200) == "Groom", "each one's partner");

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

int main()
{
	try
	{
		reads_the_couples_record();
		ring_boxes_and_result_texts();
	}
	catch (const std::exception& error)
	{
		std::cerr << "marriage: " << error.what() << "\n";
		return 1;
	}

	std::cout << "marriage: all passed\n";
	return 0;
}
