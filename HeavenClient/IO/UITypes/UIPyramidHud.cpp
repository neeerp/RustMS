#include "UIPyramidHud.h"

#include "UIDojoHud.h"

#include "../UI.h"

#include "../../Gameplay/Stage.h"
#include "../../Util/Misc.h"
#include "../../Util/NxFiles.h"

#include <nlnx/nx.hpp>

namespace ms
{
	namespace
	{
		// Where 0x55f33f puts each layer, on the screen.
		const Point<int16_t> COUNT(1, 21);
		const Point<int16_t> KEY(7, 135);
		const Point<int16_t> GAUGE(251, 64);
		const Point<int16_t> DANGER(251, 85);
		const int16_t PIXEL_Y = 71;
		const int16_t ICON_Y = 63;

		// The clock's window (0x560389), and where its minutes and seconds go in it (0x5605f2).
		const Point<int16_t> CLOCK(304, 5);
		const Point<int16_t> MINUTES(20, 13);
		const Point<int16_t> SECONDS(105, 13);

		void load_font(std::array<Texture, 10> &font, nl::node src)
		{
			for (size_t i = 0; i < font.size(); i++)
				font[i] = src[std::to_string(i)];
		}
	}

	UIPyramidHud::UIPyramidHud() : UIElement(Point<int16_t>(), Point<int16_t>()), last_skill(0), width(0),
								   in_danger(false), cleared(false)
	{
		nl::node map = NxHelper::Map::get_map_body(Stage::get().get_mapid())["mobMassacre"];
		skill_disabled = map["disableSkill"].get_integer() != 0;
		total = map["gauge"]["total"];
		decrease = map["gauge"]["decrease"];

		nl::node src = NxFiles::ui_classic["UIWindow.img"]["MonsterKilling"];
		nl::node count = src["Count"];

		// The taller panel leaves room for the skill's uses and its key.
		count_backgrd = count[skill_disabled ? "backgrd0" : "backgrd1"];
		load_font(number, count["number"]);
		load_font(number2, count["number2"]);
		key_open = count["keyBackgrd"]["open"];
		key_ing = count["keyBackgrd"]["ing"];
		key_close = count["keyBackgrd"]["close"];

		nl::node gauge = src["Gauge"];
		backgrd = gauge["backgrd"];
		text = gauge["text"];
		pixel = gauge["pixel"];
		backgrd_danger = gauge["backgrdD"];
		text_danger = gauge["textD"];
		danger = gauge["danger"];
		icon = gauge["iconD"];

		nl::node killing = nl::nx::map["Obj"]["etc.img"]["killing"];
		clock_backgrd = killing["backgrnd"];
		load_font(clock_font, killing["fontTime"]);
	}

	void UIPyramidHud::show_on(int32_t fieldtype)
	{
		if (fieldtype == massacre::FIELD_STAGE)
		{
			if (!UI::get().get_element<UIPyramidHud>())
				UI::get().emplace<UIPyramidHud>();

			// Its clock is the field's only one: it takes over from one that came as the field loaded.
			UI::get().remove(UIElement::Type::FIELDCLOCK);
		}
		else
		{
			UI::get().remove(TYPE);
		}
	}

	void UIPyramidHud::draw_number(const std::array<Texture, 10> &font, Point<int16_t> pos, int32_t slots,
								   int16_t advance, int32_t value) const
	{
		for (auto [slot, digit] : massacre::digits(value, slots))
			font[digit].draw(pos + Point<int16_t>(static_cast<int16_t>(advance * slot), 0));
	}

	void UIPyramidHud::draw(float alpha) const
	{
		const massacre::Session &state = massacre::session();

		count_backgrd.draw(position + COUNT);

		std::array<int32_t, 3> counts = { state.hit, state.miss, state.cool };

		for (size_t i = 0; i < counts.size(); i++)
		{
			Point<int16_t> at(massacre::COUNT_X, massacre::COUNT_Y[i]);
			draw_number(number, position + COUNT + at, massacre::COUNT_SLOTS, massacre::COUNT_ADVANCE, counts[i]);
		}

		if (!skill_disabled)
		{
			Point<int16_t> at(massacre::SKILL_X, massacre::SKILL_Y);
			draw_number(number2, position + COUNT + at, 1, massacre::SKILL_ADVANCE, state.skill);

			switch (key.get())
			{
				case massacre::KeyPanel::State::OPENING:
					key_open.draw(position + KEY, alpha);
					break;
				case massacre::KeyPanel::State::SHOWN:
					key_ing.draw(position + KEY, alpha);
					break;
				case massacre::KeyPanel::State::CLOSING:
					key_close.draw(position + KEY, alpha);
					break;
				default:
					break;
			}
		}

		backgrd.draw(position + GAUGE);

		if (in_danger)
			backgrd_danger.draw(position + GAUGE, alpha);

		if (width > 0)
		{
			Point<int16_t> left(massacre::GAUGE_RIGHT - width, PIXEL_Y);
			pixel.draw(DrawArgument(position + left, Point<int16_t>(width, 0)));
		}

		if (in_danger)
		{
			text_danger.draw(position + GAUGE, alpha);
			danger.draw(position + DANGER, alpha);
			icon.draw(position + Point<int16_t>(massacre::ICON_RIGHT - width, ICON_Y), alpha);
		}
		else
		{
			text.draw(position + GAUGE);
		}

		if (state.clock.is_shown())
		{
			clock_backgrd.draw(position + CLOCK);

			auto shown = massacre::clock_text(state.clock.get_reading());
			std::array<Point<int16_t>, 2> starts = { MINUTES, SECONDS };

			for (size_t i = 0; i < shown.size(); i++)
			{
				Point<int16_t> at = position + CLOCK + starts[i];

				for (char c : shown[i])
				{
					const Texture &digit = clock_font[c - '0'];
					digit.draw(at);
					at.shift_x(digit.width());
				}
			}
		}
	}

	void UIPyramidHud::update()
	{
		UIElement::update();

		massacre::Session &state = massacre::session();
		state.clock.tick(UIDojoHud::now());

		// The key opens with the first use of Rage of Pharaoh and closes after the last (0x560a97).
		if (!skill_disabled && state.skill != last_skill)
		{
			key.skill_changed(last_skill, state.skill);
			last_skill = state.skill;

			if (key.get() == massacre::KeyPanel::State::OPENING)
				key_open.play(Animation::End::HOLD);
			else if (key.get() == massacre::KeyPanel::State::CLOSING)
				key_close.play(Animation::End::HOLD);
		}

		key_open.update();
		key_ing.update();
		key_close.update();

		// Each waits for the one before to play through (0x560e85).
		if ((key.get() == massacre::KeyPanel::State::OPENING && !key_open.is_playing())
			|| (key.get() == massacre::KeyPanel::State::CLOSING && !key_close.is_playing()))
			key.finished();

		int32_t left = state.clock.is_shown() ? state.clock.get_reading() : state.length;
		int32_t value = massacre::gauge(state.length, left, decrease, state.count, total);
		width = massacre::gauge_width(value, total);
		in_danger = massacre::danger(value, total);

		backgrd_danger.update();
		text_danger.update();
		danger.update();
		icon.update();

		// The stage's "Clear", once, as its time runs out.
		if (!cleared && state.clock.is_shown() && state.clock.get_reading() <= 1)
		{
			Stage::get().add_effect("killing/clear");
			cleared = true;
		}
	}

	bool UIPyramidHud::is_in_range(Point<int16_t>) const
	{
		return false;
	}

	UIElement::Type UIPyramidHud::get_type() const
	{
		return TYPE;
	}
}
