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

		std::cout << "Field effect rules passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
