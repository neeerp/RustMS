#include "../Gameplay/LieDetector.h"
#include "../Graphics/Jpeg.h"

#define STB_IMAGE_IMPLEMENTATION
#include "stb_image.h"
#define STB_IMAGE_WRITE_IMPLEMENTATION
#include "stb_image_write.h"

#include <cstdlib>
#include <iostream>
#include <stdexcept>

namespace
{
	void require(bool condition, const char* message)
	{
		if (!condition)
			throw std::runtime_error(message);
	}

	namespace ld = ms::lie_detector;

	void append(void* context, void* data, int size)
	{
		auto out = static_cast<std::vector<uint8_t>*>(context);
		auto bytes = static_cast<uint8_t*>(data);
		out->insert(out->end(), bytes, bytes + size);
	}
}

int main()
{
	try
	{
		// FUN_007e8a2f's faces: a refusal admin/1, a fail admin/2, a pass or a fine admin/0.
		require(ld::result(ld::NOT_FOUND, 1)->portrait == 1, "not found looks neutral");
		require(ld::result(ld::NOT_ATTACKING, 1)->lines[0] ==
				"You cannot use it on a user that isn't in the middle of attack.", "0xc57");
		require(ld::result(ld::IN_A_TEST, 1)->portrait == 1, "in a test looks neutral");
		require(ld::result(ld::FAILED, 1)->portrait == 2 && ld::result(ld::FAILED, 1)->lines.size() == 1,
				"a fail looks stern, in one paragraph");
		require(ld::result(ld::FAILED, 2)->lines.size() == 2, "the GM's fail adds the sanction");
		require(ld::result(ld::PASSED, 1)->lines[0].find("5000 mesos") != std::string::npos, "a pass pays 5000");
		require(ld::result(ld::PASSED, 3)->lines[0].find("succesfully") != std::string::npos, "code 3 says passed");
		require(ld::result(ld::FINE_RECEIVED, 1)->portrait == 0, "the tester smiles at a fine");
		require(!ld::result(ld::USED, 1) && !ld::result(ld::CHALLENGE, 1) && !ld::result(4, 2),
				"chat lines and the challenge have no result dialog");
		require(!ld::result(ld::SUSPECTED, 2) && !ld::result(ld::CLEARED, 2), "nor do the GM's");

		// The chat lines: the item's names the tester; the skill's name the player tested, after whom the
		// shot that goes with most of them is named.
		require(ld::named(ld::MONITORED) && ld::named(ld::USED) && ld::named(ld::SUSPECTED) &&
				ld::named(ld::CLEARED) && !ld::named(ld::CHALLENGE) && !ld::named(ld::FINE_RECEIVED),
				"four types carry a name");
		auto used = ld::line(ld::USED, ld::BY_ITEM, "Ann");
		require(used && used->text == "Ann have used the Lie Detector Test." && !used->screenshot,
				"the item's line names the tester, unshot");
		auto activated = ld::line(ld::USED, ld::BY_SKILL, "Bo");
		require(activated && activated->text == "Bo_The screenshot has been saved. The Lie Detector has been "
												 "activated." && activated->screenshot, "the skill's is shot");
		require(ld::line(ld::SUSPECTED, ld::BY_SKILL, "Bo")->screenshot, "a fail is shot");
		auto cleared = ld::line(ld::CLEARED, ld::BY_SKILL, "Bo");
		require(cleared && cleared->text == "Bo_You have passed the Lie Detector Test." && !cleared->screenshot,
				"a pass is not");
		require(ld::line(ld::MONITORED, 7, "Bo")->screenshot, "monitoring is shot whatever the code");
		require(!ld::line(ld::USED, 3, "Bo") && !ld::line(ld::SUSPECTED, ld::BY_ITEM, "Bo") &&
				!ld::line(ld::CLEARED, ld::BY_ITEM, "Bo"), "other codes show nothing");

		// The clock: m,ss of the seconds left, hidden past two minutes or below zero.
		auto minute = ld::clock(60000);
		require(minute && (*minute == std::array<uint8_t, 4>{ 0, 1, 0, 0 }), "a minute shows 01,00");
		auto left = ld::clock(42999);
		require(left && (*left == std::array<uint8_t, 4>{ 0, 0, 4, 2 }), "42.999 s shows 00,42");
		require(ld::clock(0).has_value(), "no time left still shows");
		require(!ld::clock(-1) && !ld::clock(121000), "outside 0 to 120 s nothing shows");

		require(ld::tries(true) == 2 && ld::tries(false) == 1, "two tries, then one");

		// A first picture starts a minute; the next one keeps what was left.
		ld::time_left = 0;
		require(!ld::testing(), "no test yet");
		require(ld::picture_time(true) == ld::TEST_TIME && ld::testing(), "a first picture starts the clock");
		ld::time_left = 21000;
		require(ld::picture_time(false) == 21000, "the next picture takes the time left");

		// The picture: a baseline JPEG the server draws, decoded to BGRA.
		std::vector<uint8_t> rgb(4 * 2 * 3);
		for (size_t i = 0; i < rgb.size(); i += 3)
		{
			rgb[i] = 250;
			rgb[i + 1] = 0;
			rgb[i + 2] = 0;
		}
		std::vector<uint8_t> jpeg;
		require(stbi_write_jpg_to_func(append, &jpeg, 4, 2, 3, rgb.data(), 95) != 0, "the fixture encodes");

		int16_t width = 0;
		int16_t height = 0;
		std::vector<uint8_t> pixels;
		require(ms::decode_jpeg(jpeg, width, height, pixels), "a JPEG decodes");
		require(width == 4 && height == 2 && pixels.size() == 4 * 2 * 4, "at its size, four bytes a pixel");
		require(pixels[2] > 200 && pixels[0] < 60 && pixels[3] == 255, "red lands in BGRA's third byte, opaque");

		require(!ms::decode_jpeg({ 1, 2, 3 }, width, height, pixels) && width == 4, "junk doesn't decode");
		require(!ms::decode_jpeg({}, width, height, pixels), "nothing doesn't decode");
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << std::endl;
		return EXIT_FAILURE;
	}

	return EXIT_SUCCESS;
}
