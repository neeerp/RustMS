#include "../Gameplay/Combat/ComboOrbs.h"

#include <cmath>
#include <iostream>
#include <stdexcept>

void require(bool condition, const char* message)
{
	if (!condition) throw std::runtime_error(message);
}

bool near(float a, float b)
{
	return std::fabs(a - b) < 0.001f;
}

int main()
{
	try
	{
		using ms::combo::Orbs;

		require(Orbs::slot(1).x == 12 && Orbs::slot(1).y == 38, "the first slot");
		require(Orbs::slot(5).x == 39 && Orbs::slot(5).y == 0, "the fifth, cut down from 40");
		require(Orbs::slot(8).x == -32 && Orbs::slot(8).y == -23, "orbs five apart share a slot");
		require(!Orbs::advanced(5) && Orbs::advanced(6), "Advanced Combo's sprites from the sixth");
		require(Orbs::state(1) == 1 && Orbs::state(5) == 3 && Orbs::state(6) == 3 && Orbs::state(10) == 5,
				"two orbs to a sprite");

		{
			Orbs orbs;
			orbs.set(-1, true);
			for (size_t c = 0; c < Orbs::LAYERS; c++)
				require(!orbs.shown(c), "no Combo, nothing");

			// The first cast fades layers that were never shown.
			orbs.set(0, false);
			for (size_t c = 0; c < Orbs::LAYERS; c++)
				require(!orbs.shown(c), "no orbs, nothing");
			require(orbs.get_count() == 0, "it keeps the count");

			orbs.set(1, false);
			require(orbs.shown(Orbs::CENTER) && !orbs.shown(Orbs::ADVANCED_CENTER), "Combo Attack's center");
			require(orbs.shown(Orbs::orb(1)) && !orbs.shown(Orbs::orb(2)), "one orb");
			require(near(orbs.scale(Orbs::orb(1)), 3.0f) && near(orbs.opacity(Orbs::orb(1)), 64.0f / 255),
					"it pops in from three times its size, faint");

			orbs.update(150);
			require(near(orbs.scale(Orbs::orb(1)), 2.0f), "halfway there");
			orbs.update(150);
			require(near(orbs.scale(Orbs::orb(1)), 1.0f) && near(orbs.opacity(Orbs::orb(1)), 1.0f), "in place");

			// Two at once: only the newest pops in.
			orbs.set(3, false);
			require(near(orbs.scale(Orbs::orb(2)), 1.0f) && near(orbs.scale(Orbs::orb(3)), 3.0f),
					"only the newest pops in");
			orbs.update(300);

			orbs.set(7, false);
			for (int32_t j = 1; j <= Orbs::MOST; j++)
				require(orbs.shown(Orbs::orb(j)) == (j >= 3 && j <= 7), "the newest five");
			require(!orbs.shown(Orbs::CENTER) && orbs.shown(Orbs::ADVANCED_CENTER), "Advanced Combo's center");
			orbs.update(300);

			// A finisher fades everything, one layer after another.
			orbs.set(0, false);
			orbs.update(100);
			require(near(orbs.opacity(Orbs::ADVANCED_CENTER), 1.0f - 100.0f / 260), "the center fades");
			require(near(orbs.opacity(Orbs::orb(4)), 1.0f - 100.0f / 500), "the orbs after it");
			require(near(orbs.opacity(Orbs::orb(5)), 1.0f - 100.0f / 200), "from 200 again at the seventh layer");
			require(near(orbs.opacity(Orbs::orb(7)), 1.0f - 100.0f / 320), "and on");
			orbs.update(400);
			for (size_t c = 0; c < Orbs::LAYERS; c++)
				require(!orbs.shown(c), "all gone");

			// And the next orb shows whole again.
			orbs.set(1, false);
			orbs.update(300);
			require(orbs.shown(Orbs::CENTER) && near(orbs.opacity(Orbs::orb(1)), 1.0f), "back again");
		}

		{
			// Coming into view shows what is there, without a pop.
			Orbs orbs;
			orbs.set(4, true);
			for (int32_t j = 1; j <= Orbs::MOST; j++)
				require(orbs.shown(Orbs::orb(j)) == (j <= 4), "the four");
			require(near(orbs.scale(Orbs::orb(4)), 1.0f), "no pop");

			// Cancelled mid-pop: the others go at once, the popping one once it is in.
			orbs.set(5, false);
			orbs.update(100);
			orbs.set(-1, false);
			require(!orbs.shown(Orbs::CENTER) && !orbs.shown(Orbs::orb(1)), "gone at once");
			require(orbs.shown(Orbs::orb(5)), "but the popping one");
			orbs.update(100);
			require(orbs.shown(Orbs::orb(5)), "until it is in");
			orbs.update(100);
			require(!orbs.shown(Orbs::orb(5)), "then it goes");
		}

		{
			Orbs orbs;
			require(near(orbs.ring_angle(), 0.0f) && near(orbs.center_angle(), 0.0f), "they start square");
			orbs.update(500);
			require(near(orbs.ring_angle(), -6.2831853f / 4), "the ring turns a quarter in 500 ms");
			require(near(orbs.center_angle(), 6.2831853f / 2), "the center half, the other way");
		}

		std::cout << "Combo orbs passed\n";
		return 0;
	}
	catch (const std::exception &error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
