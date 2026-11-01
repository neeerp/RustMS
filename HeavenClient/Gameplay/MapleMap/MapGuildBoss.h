#pragma once

#include "../../Graphics/Animation.h"

namespace ms
{
	// What Ergoth's room (info/fieldType 8) shows: the chandelier where the
	// server last hung it, and the pulley (CField_GuildBoss, v83ref b380).
	class MapGuildBoss
	{
	public:
		MapGuildBoss(nl::node src);

		MapGuildBoss();

		bool is_active() const;

		void update();

		// Both pictures have z 0, in front of the field's layers and its
		// characters (QUESTIONS b379).
		void draw(double viewx, double viewy, float alpha) const;

		// A swing hit the pulley: its three frames play once, unless they
		// still are (A 5586b1).
		void swing();

	private:
		bool active = false;
		Texture healer;
		Animation pulley;
	};
}
