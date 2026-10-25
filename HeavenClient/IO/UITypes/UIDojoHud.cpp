#include "UIDojoHud.h"

#include "../UI.h"

#include "../../Audio/Audio.h"
#include "../../Gameplay/Combat/DojoEnergy.h"
#include "../../Gameplay/Stage.h"
#include "../../Util/NxFiles.h"

#include <chrono>

#include <nlnx/nx.hpp>

namespace ms
{
	namespace
	{
		// Where 0x554005 puts each layer, on the screen.
		const Point<int16_t> PLAYER_FRAME(169, 50);
		const Point<int16_t> CLOCK_FRAME(400, 26);
		const Point<int16_t> MONSTER_FRAME(631, 50);
		const Point<int16_t> EMPTY(20, 130);
		const Point<int16_t> GAGE(17, 86);
		const Point<int16_t> FULL(9, 80);

		// The clock's 112x28 canvas (0x55522b), whose glyphs go by their top left corners: the minutes' digits,
		// the bar, then the seconds'.
		const Point<int16_t> CLOCK(345, 16);
		const std::array<int16_t, 4> DIGIT_X = { 0, 23, 68, 91 };
		const Point<int16_t> BAR(51, 4);
	}

	UIDojoHud::UIDojoHud() : UIElement(Point<int16_t>(), Point<int16_t>()), player_width(0), monster_width(-1)
	{
		nl::node src = NxFiles::ui_classic["UIWindow.img"]["muruengRaid"];

		player_frame = src["player"]["0"];
		clock_frame = src["clock"]["0"];
		monster_frame = src["monster"]["0"];
		// Both gauges are drawn from the player's (0x5544a0).
		hp_gage = src["player"]["Gage"]["0"];

		for (size_t i = 0; i < digits.size(); i++)
			digits[i] = src["number"][std::to_string(i)];

		bar = src["number"]["bar"];

		nl::node energy = src["energy"];
		empty = energy["empty"]["0"];
		gage = energy["empty"]["Gage"]["0"];
		full = energy["full"];
	}

	void UIDojoHud::show_on(int32_t fieldtype)
	{
		if (pq_skills::allowed(pq_skills::Kind::DOJO_BUFF, fieldtype))
		{
			if (!UI::get().get_element<UIDojoHud>())
				UI::get().emplace<UIDojoHud>();

			// Its clock is the field's only one: it takes over from one that came as the field loaded.
			UI::get().remove(UIElement::Type::FIELDCLOCK);
		}
		else
		{
			UI::get().remove(TYPE);
		}
	}

	int64_t UIDojoHud::now()
	{
		auto since = std::chrono::steady_clock::now().time_since_epoch();

		return std::chrono::duration_cast<std::chrono::milliseconds>(since).count();
	}

	void UIDojoHud::draw(float alpha) const
	{
		player_frame.draw(position + PLAYER_FRAME);
		clock_frame.draw(position + CLOCK_FRAME);
		monster_frame.draw(position + MONSTER_FRAME);

		if (player_width > 0)
		{
			Point<int16_t> left(dojo_energy::PLAYER_GAUGE_RIGHT - player_width, dojo_energy::GAUGE_Y);
			hp_gage.draw(DrawArgument(position + left, Point<int16_t>(player_width, 0)));
		}

		if (monster_width > 0)
		{
			Point<int16_t> left(dojo_energy::MONSTER_GAUGE_LEFT, dojo_energy::GAUGE_Y);
			hp_gage.draw(DrawArgument(position + left, Point<int16_t>(monster_width, 0)));
		}

		const dojo_energy::Countdown &clock = dojo_energy::session().clock;

		if (clock.is_shown())
		{
			bar.draw(position + CLOCK + BAR + bar.get_origin());

			if (clock.has_digits())
			{
				auto shown = dojo_energy::clock_digits(clock.get_reading());

				for (size_t i = 0; i < shown.size(); i++)
				{
					const Texture &digit = digits[shown[i]];
					digit.draw(position + CLOCK + Point<int16_t>(DIGIT_X[i], 0) + digit.get_origin());
				}
			}
		}

		int32_t energy = dojo_energy::session().energy;

		if (energy >= dojo_energy::FULL)
		{
			full.draw(position + FULL, alpha);

			return;
		}

		empty.draw(position + EMPTY);

		// The dark strip, stretched over the part not yet filled.
		if (int16_t cover = dojo_energy::cover(energy))
			gage.draw(DrawArgument(position + GAGE, Point<int16_t>(0, cover)));
	}

	void UIDojoHud::update()
	{
		UIElement::update();

		full.update();

		// The local character's HP, hidden without a maximum.
		const CharStats &stats = Stage::get().get_player().get_stats();
		player_width = dojo_energy::player_gauge(stats.get_stat(MapleStat::Id::HP), stats.get_total(EquipStat::Id::HP));

		int16_t percent = Stage::get().get_mobs().dojo_boss_percent();
		monster_width = percent < 0 ? -1 : dojo_energy::monster_gauge(percent);

		// Time over: the HUD stops at 00:00 and shows it once (0x555626).
		if (dojo_energy::session().clock.tick(now()))
		{
			Stage::get().add_effect("dojang/timeOver");
			Sound(nl::nx::sound["Field.img"]["Dojang"]["timeOver"]).play();
		}
	}

	bool UIDojoHud::is_in_range(Point<int16_t>) const
	{
		return false;
	}

	UIElement::Type UIDojoHud::get_type() const
	{
		return TYPE;
	}
}
