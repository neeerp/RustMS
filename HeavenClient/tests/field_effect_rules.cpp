#include "../Gameplay/FieldEffectRules.h"
#include <cmath>
#include <iostream>
#include <stdexcept>

void require(bool condition, const char* message)
{
	if (!condition) throw std::runtime_error(message);
}

int main()
{
	try
	{
		using namespace ms::field_effect;

		require(summon_volume(0) == 100 && summon_volume(249.9) == 100, "full within 250 px");
		require(summon_volume(250) == 100 && summon_volume(500) == 80, "120 - 0.08 d");
		require(summon_volume(301) == 95, "truncated");
		require(summon_volume(1000) == 40 && summon_volume(5000) == 40, "40 from 1000 px");

		ScreenPosition classic = screen_position(800, 600);
		require(classic.x == 400 && classic.y == 260, "(400, 260) in 800x600");
		ScreenPosition wide = screen_position(1024, 768);
		require(wide.x == 512 && wide.y == 344, "40 px above the centre");

		require(bgm_path("Bgm09/TimeAttack") == "Bgm09.img/TimeAttack", "the img holds the track");
		require(bgm_path("Jukebox/Congratulation") == "Jukebox.img/Congratulation", "the jukebox's too");
		require(bgm_path("Bgm09").empty(), "no track");

		require(fade_gain(0, BGM_FADE) == 1.0f && fade_gain(600, BGM_FADE) == 0.0f, "full to silent");
		require(std::abs(fade_gain(300, BGM_FADE) - 0.53f) < 0.01f, "half the volume is 5.5 dB down");
		require(fade_gain(450, BGM_FADE) < 0.2f && fade_gain(450, BGM_FADE) > fade_gain(540, BGM_FADE), "falls away");

		require(fade_in_gain(0, 1000) == 0.0f && fade_in_gain(1000, 1000) == 1.0f, "silent to full");
		require(fade_in_gain(500, 1000) == fade_gain(500, 1000) && fade_in_gain(250, 1000) == fade_gain(750, 1000),
				"the fade backwards");
		require(fade_in_gain(5, 0) == 1.0f, "full at once without a duration");

		require(is_jukebox(5100000) && !is_jukebox(5120000) && !is_jukebox(-1), "only 510xxxx items");
		require(jukebox_path("Sound/Jukebox.img/Congratulation") == "Jukebox.img/Congratulation", "from Sound.nx's top");
		require(jukebox_path("Jukebox.img/Congratulation") == "Jukebox.img/Congratulation", "already there");
		require(jukebox_line("%s's song for you.", "Writs") == "Writs's song for you.", "the name for %s");
		require(jukebox_line("A song for you.", "Writs") == "A song for you.", "no %s");

		uint8_t header[92] = {};
		header[60] = 0x10;
		header[61] = 0x27;
		require(sound_length(header, 92) == 1, "10 bytes at 10000 a second");
		require(sound_length(header, 82) == 0 && sound_length(nullptr, 92) == 0, "no sound");
		header[60] = header[61] = 0;
		require(sound_length(header, 92) == 0, "no rate");

		require(back_target(0) == 255.0f && back_target(1) == 0.0f, "in to full, out to nothing");
		require(!back_target(2) && !back_target(255), "other modes do nothing");

		require(back_alpha(255, 0, 0, 1000) == 255 && back_alpha(255, 0, 250, 1000) == 191.25f, "linear");
		require(back_alpha(100, 255, 1000, 1000) == 255 && back_alpha(100, 255, 1200, 1000) == 255, "then held");
		require(back_alpha(0, 255, 0, 0) == 255, "at once without a duration");

		std::cout << "Field effect rules passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
