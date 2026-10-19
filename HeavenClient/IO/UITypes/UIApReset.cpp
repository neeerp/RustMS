#include "UIApReset.h"

#include "../KeyAction.h"

#include "../Components/MapleButton.h"

#include "../../Constants.h"
#include "../../Gameplay/Stage.h"
#include "../../Util/NxFiles.h"

#include <nlnx/nx.hpp>

#include <algorithm>

namespace ms
{
	namespace
	{
		// Each dialog's size; FUN_004edaef centres it.
		constexpr int16_t WIDTH = 175, HEIGHT = 220, CONFIRM_HEIGHT = 141;
		// The rest is from the dialog's top left (FUN_008cb99e, FUN_008cc3b8, FUN_008cd6f8, FUN_008cd8a0). v83
		// draws a line's cap top about 3 below where it is drawn, we 9, so text goes up by the difference.
		constexpr int16_t TEXT_RAISE = 6;
		constexpr int16_t ROW_Y[ap_reset::NUM_ROWS] = { 72, 90, 114, 132, 150, 168 };
		constexpr int16_t BUTTON_X = 153, VALUE_X = 61, LABEL_X = 8, BY_GAP = 4;
		const Point<int16_t> CLOSE_AT(156, 6);
		const Point<int16_t> OK_AT(70, 117);
		const Point<int16_t> CANCEL_AT(119, 117);
		// The confirm dialog's rows: where the AP goes, then where it left.
		constexpr int16_t TO_Y = 72, FROM_Y = 90;

		int16_t button_y(int32_t row)
		{
			return ROW_Y[row] + 2;
		}
	}

	UIApReset::UIApReset(std::function<void(int32_t, int32_t)> oc)
			: UIElement(Point<int16_t>(), Point<int16_t>(WIDTH, HEIGHT)), onconfirm(std::move(oc)), step(SOURCE),
			  from(ap_reset::HP), to(ap_reset::HP)
	{
		nl::node ap = NxFiles::ui_classic["UIWindow.img"]["Reset"]["AP"];

		for (int32_t row = 0; row < ap_reset::NUM_ROWS; row++)
		{
			labels[row] = ap["stat0"][std::to_string(row)];
			grey_labels[row] = ap["stat1"][std::to_string(row)];
			values[row] = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK);
		}

		less = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLUE);
		less_by = Text(Text::Font::A12B, Text::Alignment::LEFT, Color::Name::BLUE);
		more = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::RED);
		more_by = Text(Text::Font::A12B, Text::Alignment::LEFT, Color::Name::RED);

		show(SOURCE);
	}

	void UIApReset::draw(float alpha) const
	{
		UIElement::draw_sprites(alpha);

		auto at = [&](int16_t x, int16_t y)
		{
			return position + Point<int16_t>(x, y - TEXT_RAISE);
		};

		auto with_change = [&](const Text &value, const Text &by, int16_t y)
		{
			value.draw(at(VALUE_X, y));
			by.draw(at(VALUE_X + value.width() + BY_GAP, y));
		};

		if (step == CONFIRM)
		{
			labels[to].draw(position + Point<int16_t>(LABEL_X, TO_Y - 1));
			with_change(more, more_by, TO_Y);
			labels[from].draw(position + Point<int16_t>(LABEL_X, FROM_Y - 1));
			with_change(less, less_by, FROM_Y);
		} else
		{
			for (int32_t row = 0; row < ap_reset::NUM_ROWS; row++)
			{
				if (step == TARGET && row == from)
				{
					grey_labels[row].draw(position + Point<int16_t>(LABEL_X, ROW_Y[row] - 1));
					with_change(less, less_by, ROW_Y[row]);
				} else
				{
					values[row].draw(at(VALUE_X, ROW_Y[row]));
				}
			}
		}

		UIElement::draw_buttons(alpha);
	}

	void UIApReset::update()
	{
		UIElement::update();

		// FUN_008cc3b8 reads the stats each time it draws.
		ap_reset::Stats s = stats();

		auto set = [](Text &text, const std::string &str)
		{
			if (text.get_text() != str)
				text.change_text(str);
		};

		for (int32_t row = 0; row < ap_reset::NUM_ROWS; row++)
			set(values[row], std::to_string(ap_reset::value(s, static_cast<ap_reset::Row>(row))));

		set(less, std::to_string(ap_reset::value(s, from) - ap_reset::loss(s, from)));
		set(less_by, "(-" + std::to_string(ap_reset::loss(s, from)) + ")");
		set(more, std::to_string(ap_reset::value(s, to) + ap_reset::gain(s, to)));
		set(more_by, "(+" + std::to_string(ap_reset::gain(s, to)) + ")");
	}

	void UIApReset::update_screen(int16_t new_width, int16_t new_height)
	{
		int16_t x = std::clamp<int16_t>(new_width / 2 - dimension.x() / 2, 0,
										std::max<int16_t>(new_width - dimension.x(), 0));
		int16_t y = std::clamp<int16_t>(new_height / 2 - dimension.y() / 2, 0,
										std::max<int16_t>(new_height - dimension.y(), 0));

		position = Point<int16_t>(x, y);
	}

	void UIApReset::send_key(int32_t keycode, bool pressed, bool escape)
	{
		if (!pressed)
			return;

		// Only the confirm dialog has an OK for Enter (FUN_004edd21).
		if (escape)
			deactivate();
		else if (keycode == KeyAction::Id::RETURN && step == CONFIRM)
			confirm();
	}

	UIElement::Type UIApReset::get_type() const
	{
		return TYPE;
	}

	Button::State UIApReset::button_pressed(uint16_t buttonid)
	{
		if (buttonid == Buttons::CLOSE || buttonid == Buttons::CANCEL)
		{
			deactivate();
		} else if (buttonid == Buttons::OK)
		{
			confirm();
		} else if (buttonid < Buttons::TARGET_ROW)
		{
			from = static_cast<ap_reset::Row>(buttonid - Buttons::SOURCE_ROW);
			show(TARGET);
		} else if (buttonid < Buttons::CLOSE)
		{
			to = static_cast<ap_reset::Row>(buttonid - Buttons::TARGET_ROW);
			show(CONFIRM);
		}

		return Button::State::NORMAL;
	}

	void UIApReset::show(Step next)
	{
		step = next;

		nl::node reset = NxFiles::ui_classic["UIWindow.img"]["Reset"];
		nl::node basic = NxFiles::ui_classic["Basic.img"];

		sprites.clear();
		buttons.clear();

		buttons[Buttons::CLOSE] = std::make_unique<MapleButton>(basic["BtClose"], CLOSE_AT);

		if (step == CONFIRM)
		{
			dimension = Point<int16_t>(WIDTH, CONFIRM_HEIGHT);
			sprites.emplace_back(reset["AP"]["backgrnd3"]);
			buttons[Buttons::OK] = std::make_unique<MapleButton>(basic["BtOK2"], OK_AT);
			buttons[Buttons::CANCEL] = std::make_unique<MapleButton>(basic["BtCancel2"], CANCEL_AT);
		} else
		{
			// FUN_008cbddb decides once which rows may be picked.
			ap_reset::Stats s = stats();
			bool source = step == SOURCE;

			dimension = Point<int16_t>(WIDTH, HEIGHT);
			sprites.emplace_back(reset["AP"][source ? "backgrnd" : "backgrnd2"]);

			for (int32_t i = 0; i < ap_reset::NUM_ROWS; i++)
			{
				auto row = static_cast<ap_reset::Row>(i);
				uint16_t id = (source ? Buttons::SOURCE_ROW : Buttons::TARGET_ROW) + i;
				nl::node look = reset["Common"][source ? "BtDown" : "BtUP"];

				buttons[id] = std::make_unique<MapleButton>(look, Point<int16_t>(BUTTON_X, button_y(i)));

				if (source ? !ap_reset::may_take(s, row) : !ap_reset::may_give(s, row, from))
					buttons[id]->set_state(Button::State::DISABLED);
			}
		}

		update();
		update_screen(Constants::Constants::get().get_viewwidth(), Constants::Constants::get().get_viewheight());
	}

	void UIApReset::confirm()
	{
		deactivate();
		onconfirm(ap_reset::FLAGS[to], ap_reset::FLAGS[from]);
	}

	ap_reset::Stats UIApReset::stats() const
	{
		const CharStats &stats = Stage::get().get_player().get_stats();

		auto stat = [&](MapleStat::Id id)
		{
			return static_cast<int32_t>(stats.get_stat(id));
		};

		return { stat(MapleStat::Id::JOB), stat(MapleStat::Id::LEVEL), stat(MapleStat::Id::STR),
				 stat(MapleStat::Id::DEX), stat(MapleStat::Id::INT), stat(MapleStat::Id::LUK),
				 stat(MapleStat::Id::AP), stat(MapleStat::Id::MAXHP), stat(MapleStat::Id::MAXMP),
				 stats.get_total(EquipStat::Id::INT) };
	}
}
