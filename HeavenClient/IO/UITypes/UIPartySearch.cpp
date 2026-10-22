#include "UIPartySearch.h"

#include "UINotice.h"
#include "UIUserList.h"

#include "../KeyAction.h"
#include "../PartySearchActions.h"
#include "../UI.h"

#include "../Components/MapleButton.h"

#include "../../Gameplay/Stage.h"
#include "../../Util/NxFiles.h"

#include <nlnx/nx.hpp>

#include <cstdlib>

namespace ms
{
	namespace
	{
		const Point<int16_t> LAYOUT_AT[] = { { 6, 53 }, { 6, 84 }, { 9, 95 }, { 9, 130 }, { 9, 151 } };

		constexpr int16_t BUTTON_Y = 373;
		const int16_t BUTTON_X[] = { 5, 73, 141 };

		// The edits for the lowest and highest level and the party size,
		// 25 by 15 on a white fill, their text 2 px up (param +8).
		// HeavenClient draws text 5 lower than v83.
		const Point<int16_t> FIELD_AT[] = { { 54, 98 }, { 114, 98 }, { 240, 98 } };
		constexpr int16_t FIELD_WIDTH = 25;
		constexpr int16_t FIELD_HEIGHT = 15;
		constexpr size_t FIELD_LIMIT[] = { 3, 3, 1 };

		// Each box's control: the box 2 right and 1 down in it, the label
		// 18 right and 2 down, and a click anywhere along both. HeavenClient
		// draws Arial 11 8 lower than v83.
		const Point<int16_t> BOX_AT[PartySearch::BOXES] = {
			{ 10, 131 }, { 10, 153 }, { 10, 206 },
			{ 94, 153 }, { 94, 173 }, { 94, 193 }, { 94, 213 }, { 94, 233 },
			{ 186, 153 }, { 186, 173 }, { 186, 193 }, { 186, 213 }, { 186, 233 },
			{ 10, 258 }, { 10, 278 }, { 10, 298 }, { 10, 318 },
			{ 94, 258 }, { 94, 278 }, { 94, 298 }, { 94, 318 },
			{ 186, 258 }, { 186, 278 }, { 186, 298 }, { 186, 318 }
		};
		const Point<int16_t> CHECK_IN(2, 1);
		const Point<int16_t> LABEL_IN(18, 2 - 8);
		constexpr int16_t BOX_HEIGHT = 15;
		constexpr int16_t BOX_PAST_LABEL = 20;

		constexpr const char* NOT_LEADER = "You are not the master of the party.";

		// v83's notices break lines with "\r\n", ours with "\\n".
		std::string as_notice(std::string text)
		{
			for (size_t at = text.find("\r\n"); at != std::string::npos; at = text.find("\r\n", at))
				text.replace(at, 2, "\\n");

			return text;
		}

		int16_t level()
		{
			return Stage::get().get_player().get_stats().get_stat(MapleStat::Id::LEVEL);
		}
	}

	UIPartySearch::UIPartySearch() : UIDragElement<PosPARTYSEARCH>(Point<int16_t>(305, 20))
	{
		nl::node src = NxFiles::ui_classic["UIWindow.img"]["PartySearch"];
		nl::node basic = NxFiles::ui_classic["Basic.img"];

		background = src["backgrnd"];
		dimension = background.get_dimensions();

		for (size_t i = 0; i < layout.size(); i++)
			layout[i] = src["PartyMember"]["party" + std::to_string(i)];

		for (size_t i = 0; i < checkbox.size(); i++)
			checkbox[i] = basic["CheckBox"][std::to_string(i)];

		buttons[START] = std::make_unique<MapleButton>(src["BtStart"], Point<int16_t>(BUTTON_X[0], BUTTON_Y));
		buttons[PAUSE] = std::make_unique<MapleButton>(src["BtPause"], Point<int16_t>(BUTTON_X[1], BUTTON_Y));
		buttons[STOP] = std::make_unique<MapleButton>(src["BtStop"], Point<int16_t>(BUTTON_X[2], BUTTON_Y));

		field_fill = ColorBox(FIELD_WIDTH, FIELD_HEIGHT, Color::Name::WHITE, 1.0f);

		for (size_t i = 0; i < fields.size(); i++)
		{
			Point<int16_t> at = FIELD_AT[i];
			Rectangle<int16_t> bounds(at.x(), at.x() + FIELD_WIDTH, at.y() - 7, at.y() + FIELD_HEIGHT - 7);

			fields[i] = Textfield(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, bounds, FIELD_LIMIT[i]);
			fields[i].set_digits_only();
			fields[i].set_shown_disabled();
		}

		// Tab moves to the next edit, and Esc empties this one (0x878e61).
		for (size_t i = 0; i < fields.size(); i++)
		{
			Textfield& next = fields[(i + 1) % fields.size()];

			fields[i].set_key_callback(KeyAction::Id::TAB, [this, i, &next]()
			{
				fields[i].set_state(Textfield::State::NORMAL);
				next.set_state(Textfield::State::FOCUSED);
			});
			fields[i].set_key_callback(KeyAction::Id::ESCAPE, [this, i]()
			{
				fields[i].change_text("");
			});
		}

		load();
	}

	void UIPartySearch::toggle()
	{
		auto window = UI::get().get_element<UIPartySearch>();
		bool opening = !window || !window->is_active();

		UI::get().emplace<UIPartySearch>();

		if (!opening)
			return;

		window = UI::get().get_element<UIPartySearch>();

		if (!window)
			return;

		window->load();

		// v83 builds the window afresh each time, beside the user list if
		// that is open (0xa05f7c).
		auto userlist = UI::get().get_element<UIUserList>();

		if (userlist && userlist->is_active())
			window->position = userlist->get_position() + Point<int16_t>(userlist->get_dimension().x(), 0);
	}

	void UIPartySearch::load()
	{
		int16_t lv = level();

		for (size_t i = 0; i < PartySearch::BOXES; i++)
		{
			shown[i] = PartySearch::box_shown(i, lv);
			labels[i] = Text(Text::Font::A11M, Text::Alignment::LEFT, Color::Name::BLACK, PartySearch::box_label(i, lv));
		}

		const PartySearch& search = PartySearch::get();

		show(search.state() == PartySearch::State::IDLE ? PartySearch::defaults(lv) : search.wanted());

		seen = search.state();
		refresh();
	}

	void UIPartySearch::show(const PartySearch::Wanted& wanted)
	{
		int32_t values[] = { wanted.min_level, wanted.max_level, wanted.members };

		for (size_t i = 0; i < fields.size(); i++)
			fields[i].change_text(values[i] > 0 ? std::to_string(values[i]) : "");

		jobs = wanted.jobs;
	}

	void UIPartySearch::refresh()
	{
		PartySearch::State state = PartySearch::get().state();

		// A stop shows the defaults again (0x8793a1).
		if (state == PartySearch::State::IDLE && seen != PartySearch::State::IDLE)
			show(PartySearch::defaults(level()));

		seen = state;
		enabled = state != PartySearch::State::SEARCHING;

		buttons[START]->set_state(enabled ? Button::State::NORMAL : Button::State::DISABLED);
		buttons[PAUSE]->set_state(state == PartySearch::State::SEARCHING ? Button::State::NORMAL
				: Button::State::DISABLED);
		buttons[STOP]->set_state(state != PartySearch::State::IDLE ? Button::State::NORMAL : Button::State::DISABLED);

		for (Textfield& field : fields)
		{
			if (!enabled)
				field.set_state(Textfield::State::DISABLED);
			else if (field.get_state() == Textfield::State::DISABLED)
				field.set_state(Textfield::State::NORMAL);
		}

		for (size_t i = 0; i < PartySearch::BOXES; i++)
		{
			// Beginner and Aran stand alone, coloured as heads.
			bool dark = i == 1 || i == 2 || PartySearch::box_heads(i);
			Color::Name color = !enabled ? Color::Name::JOBDISABLED
					: dark ? Color::Name::JOBHEAD : Color::Name::SILVERCHALICE;

			labels[i].change_color(color);
		}

		pressed = PartySearch::BOXES;
	}

	void UIPartySearch::draw(float inter) const
	{
		background.draw(position);

		for (size_t i = 0; i < layout.size(); i++)
			layout[i].draw(position + LAYOUT_AT[i]);

		for (size_t i = 0; i < fields.size(); i++)
		{
			field_fill.draw(position + FIELD_AT[i]);
			fields[i].draw(position);
		}

		for (size_t i = 0; i < PartySearch::BOXES; i++)
		{
			if (!shown[i])
				continue;

			bool checked = jobs & (1 << i);

			checkbox[(checked ? 1 : 0) + (enabled ? 0 : 2)].draw(position + BOX_AT[i] + CHECK_IN);
			labels[i].draw(position + BOX_AT[i] + LABEL_IN);
		}

		UIElement::draw(inter);
	}

	void UIPartySearch::update()
	{
		UIElement::update();

		for (Textfield& field : fields)
			field.update(position);
	}

	size_t UIPartySearch::box_at(Point<int16_t> cursorpos) const
	{
		for (size_t i = 0; i < PartySearch::BOXES; i++)
		{
			if (!shown[i])
				continue;

			Point<int16_t> at = position + BOX_AT[i];
			Rectangle<int16_t> control(at, at + Point<int16_t>(labels[i].width() + BOX_PAST_LABEL, BOX_HEIGHT));

			if (control.contains(cursorpos))
				return i;
		}

		return PartySearch::BOXES;
	}

	Cursor::State UIPartySearch::send_cursor(bool clicked, Point<int16_t> cursorpos)
	{
		if (enabled && !dragged)
		{
			for (Textfield& field : fields)
			{
				Cursor::State state = field.send_cursor(cursorpos, clicked);

				if (state != Cursor::State::IDLE)
					return state;
			}

			// A box changes on the button's release (CCtrlCheckBox, 179e).
			size_t box = box_at(cursorpos);

			if (clicked && box < PartySearch::BOXES)
			{
				pressed = box;

				return Cursor::State::CLICKING;
			}

			if (!clicked && pressed < PartySearch::BOXES)
			{
				if (box == pressed)
					jobs = PartySearch::toggled(jobs, box);

				pressed = PartySearch::BOXES;
			}
		}

		return UIDragElement::send_cursor(clicked, cursorpos);
	}

	void UIPartySearch::remove_cursor()
	{
		UIDragElement::remove_cursor();

		pressed = PartySearch::BOXES;
	}

	void UIPartySearch::send_key(int32_t, bool pressed_key, bool escape)
	{
		if (pressed_key && escape)
			deactivate();
	}

	Button::State UIPartySearch::button_pressed(uint16_t buttonid)
	{
		switch (buttonid)
		{
		case START:
			start();
			break;
		case PAUSE:
			PartySearchActions::pause();
			break;
		case STOP:
			PartySearchActions::stop();
			break;
		}

		return buttons[buttonid]->get_state() == Button::State::DISABLED ? Button::State::DISABLED
				: Button::State::NORMAL;
	}

	void UIPartySearch::start()
	{
		Player& player = Stage::get().get_player();
		const Party& party = player.get_party();

		if (!party.exists() || !party.is_leader(player.get_oid()))
		{
			UI::get().emplace<UIOk>(NOT_LEADER, [](bool) {});

			return;
		}

		if (PartySearch::get().state() == PartySearch::State::SEARCHING)
			return;

		PartySearch::Wanted wanted;
		wanted.min_level = std::atoi(fields[0].get_text().c_str());
		wanted.max_level = std::atoi(fields[1].get_text().c_str());
		wanted.members = std::atoi(fields[2].get_text().c_str());
		wanted.jobs = jobs;

		if (auto refusal = PartySearch::refusal(wanted, level(), party.get_members().size()))
		{
			if (!refusal->empty())
				UI::get().emplace<UIOk>(as_notice(*refusal), [](bool) {});

			return;
		}

		PartySearchActions::start(wanted);
	}

	UIElement::Type UIPartySearch::get_type() const
	{
		return TYPE;
	}
}
