#include "../Character/ItemEffectRules.h"
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
		using namespace ms::item_effect;

		require(is_effect(5010000) && is_effect(5010999) && is_effect(4290001), "501 and 429");
		require(!is_effect(5020000) && !is_effect(4280000) && !is_effect(0), "nothing else");
		require(std::string(category(4290000)) == "Etc" && std::string(category(5010044)) == "Cash", "by kind");
		require(image(5010000) == "0501.img" && image(4290001) == "0429.img", "the item's image");

		require(toggled(0, 5010000) == 5010000, "show one");
		require(toggled(5010000, 5010000) == 0, "the one shown stops");
		require(toggled(5010000, 5010020) == 5010020, "another replaces it");

		require(may_send(5010000, 0, -1), "the first is free");
		require(!may_send(5010000, 1999, 0) && may_send(5010000, 2000, 0), "2 s after the last shown");
		require(may_send(0, 10, 0), "stopping is never held back");

		require(key(true, true, "walk1") == "walk1", "the action's node");
		require(key(true, false, "walk1") == "default", "else default");
		require(key(false, true, "walk1") == "default", "an item without action");

		// 201-item-effects section 6: loose 10, 50 px away, 16 ms later, it closes 3.7 px.
		Trail x;
		require(trail(x, 0, 100, 10) == 0 && x.started, "starts on its target");
		require(trail(x, 50, 116, 10) == 3, "lags L d / (dt d + L) behind");
		require(std::abs(x.at - (50.0 - 10000.0 * 50.0 / (16.0 * 50.0 + 10000.0))) < 1e-9, "keeps the exact value");

		Trail near;
		trail(near, 0, 0, 10);
		require(trail(near, 28, 8, 10) == 0 && trail(near, -28, 16, 10) == 0, "still within 28 px");
		require(trail(near, 29, 16, 0) == 29, "loose 0 keeps up");

		Trail left;
		trail(left, 100, 0, 10);
		require(trail(left, 0, 1000, 10) == 9, "from the other side, a second later: 9 px short");

		Trail stuck;
		trail(stuck, 0, 0, 0);
		require(trail(stuck, 100, 0, 0) == 100, "no time and loose 0");

		require(follower_faces_right(10, 10) && follower_faces_right(11, 10) && !follower_faces_right(9, 10),
			"faces the character");

		require(ghost_opacity(224, 0, 600) == 224.0f / 255.0f, "starts at alpha");
		require(std::abs(ghost_opacity(255, 300, 600) - 0.5f) < 1e-6f, "fades linearly");
		require(ghost_opacity(224, 600, 600) == 0.0f && ghost_opacity(224, 10, 0) == 0.0f, "then gone");

		std::cout << "Item effect rules passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
