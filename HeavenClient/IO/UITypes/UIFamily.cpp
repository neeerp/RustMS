#include "UIFamily.h"

#include "UINotice.h"

#include "../FamilyActions.h"
#include "../UI.h"

#include "../Components/MapleButton.h"

#include "../../Gameplay/Stage.h"
#include "../../Net/Packets/FamilyPackets.h"
#include "../../Util/Misc.h"
#include "../../Util/NxFiles.h"

#include <cstdio>

#ifdef USE_NX
#include <nlnx/nx.hpp>
#endif

namespace ms
{
	namespace
	{
		// Window-local, from b042. v83 draws a line's cap top about 3 below
		// where it is drawn, we 9, so text goes up by the difference.
		constexpr int16_t TEXT_RAISE = 6;
		const Point<int16_t> DRAG_AREA(224, 25);

		const Point<int16_t> JUNIOR_AT(95, 77), TREE_AT(164, 77), LEFT_AT(15, 177), RIGHT_AT(196, 177);
		const Point<int16_t> PRECEPTS_BUTTON_AT(209, 55), SPECIAL_AT(152, 322), OK_AT(93, 368);

		// Each text's left edge and width; centred or right-aligned in it.
		constexpr int16_t TITLE_X = 7, TITLE_Y = 33, TITLE_W = 209;
		constexpr int16_t JUNIORS_X = 52, JUNIORS_Y = 81, JUNIORS_W = 37;
		constexpr int16_t REPS_X = 54, REPS_Y = 111, REPS_W = 159;
		constexpr int16_t TODAY_X = 106, TODAY_Y = 128, TODAY_W = 107;
		constexpr int16_t NAME_X = 15, NAME_Y = 178, NAME_W = 193;
		constexpr int16_t COST_X = 74, COST_Y = 198, COST_W = 43;
		constexpr int16_t USED_X = 175, USED_Y = 198, USED_W = 32;
		constexpr int16_t PAGE_X = 158, PAGE_Y = 158;
		constexpr int16_t DESCRIPTION_X = 16, DESCRIPTION_Y = 214, DESCRIPTION_W = 195;

		// The precepts scroll through a 194x15 strip, a pixel a step, and
		// start again from its right edge once gone (0x8093fc).
		constexpr int16_t PRECEPTS_X = 9, PRECEPTS_Y = 60, PRECEPTS_W = 194;
		// I: v83 moves them each of its updates; we take one every four of
		// our 8 ms steps.
		constexpr uint16_t PRECEPTS_STEPS = 4;

		const Rectangle<int16_t> REPS_TIP(6, 217, 108, 140);
		const Rectangle<int16_t> JUNIORS_TIP(6, 93, 78, 93);
		const Rectangle<int16_t> USED_TIP(112, 209, 194, 209);

		// StringPool 0x11f1, 0x11ef, 0x11eb, 0x120f.
		constexpr const char* NO_FAMILY = "(You do not have a family yet.)";
		constexpr const char* PRECEPTS_PROMPT = "Enter the family precepts.";
		constexpr const char* TARGET_PROMPT = "Please enter the name of the targeted family member.";
		// 0x1213, 0x1214, 0x1219, 0x121a.
		constexpr const char* REPS_HELP = "Reps are marked as [Current Rep/Total Rep].\\nUse up your current Reps to acquire [Entitlements].";
		constexpr const char* JUNIORS_HELP = "Juniors are marked as [Current Juniors/Max Juniors Available].\\n When a Junior you have registered under gains EXP or levels up, your Rep will increase as well.\\nHelp your Junior level up to further increase your Rep.";
		constexpr const char* USED_UP = "You cannot use any more Entitlements today.";
		constexpr size_t NAME_LIMIT = 12;

		std::string number(int32_t value)
		{
			std::string text = std::to_string(value);
			string_format::split_number(text);

			return text;
		}

		Text line(Text::Alignment alignment, const std::string& text = "")
		{
			return Text(Text::Font::A12M, alignment, Color::Name::BLACK, text);
		}

		const Family& family()
		{
			return Stage::get().get_player().get_family();
		}

		// Inclusive of the bottom and right edges, as PtInRect is not;
		// near enough for a tooltip.
		bool inside(const Rectangle<int16_t>& rect, Point<int16_t> local)
		{
			return local.x() >= rect.left() && local.x() < rect.right()
				&& local.y() >= rect.top() && local.y() < rect.bottom();
		}
	}

	UIFamily::UIFamily() : UIDragElement<PosFAMILY>(DRAG_AREA)
	{
		nl::node src = NxFiles::ui_classic["UIWindow.img"]["Family"];

		background = src["backgrnd"];
		dimension = background.get_dimensions();

		buttons[JUNIOR] = std::make_unique<MapleButton>(src["BtJuniorEntry"], JUNIOR_AT);
		buttons[TREE] = std::make_unique<MapleButton>(src["BtTree"], TREE_AT);
		buttons[LEFT] = std::make_unique<MapleButton>(src["BtLeft"], LEFT_AT);
		buttons[RIGHT] = std::make_unique<MapleButton>(src["BtRight"], RIGHT_AT);
		buttons[PRECEPTS] = std::make_unique<MapleButton>(src["BtFamilyPrecept"], PRECEPTS_BUTTON_AT);
		buttons[SPECIAL] = std::make_unique<MapleButton>(src["BtSpecial"], SPECIAL_AT);
		buttons[OK] = std::make_unique<MapleButton>(src["BtOK"], OK_AT);

		title = line(Text::Alignment::CENTER);
		juniors = line(Text::Alignment::RIGHT);
		reps = line(Text::Alignment::RIGHT);
		today = line(Text::Alignment::RIGHT);
		name = line(Text::Alignment::CENTER);
		cost = line(Text::Alignment::RIGHT);
		used = line(Text::Alignment::RIGHT);
		page = line(Text::Alignment::LEFT);
		// One line however long, as it scrolls.
		precepts = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, "", INT16_MAX, false);

		refresh();
	}

	void UIFamily::toggle()
	{
		auto window = UI::get().get_element<UIFamily>();
		bool opening = !window || !window->is_active();

		UI::get().emplace<UIFamily>();

		if (!opening)
			return;

		// v83 builds the window afresh each time, and asks for the
		// numbers as it does (0x808a1a).
		if (auto opened = UI::get().get_element<UIFamily>())
		{
			opened->selected = 0;
			opened->precepts_x = PRECEPTS_W;
			opened->refresh();
		}

		OpenFamilyPacket().dispatch();
	}

	void UIFamily::refresh()
	{
		const FamilyInfo& info = family().get_info();

		if (info.name.empty())
			title.change_text(NO_FAMILY);
		else
			title.change_text(info.name + " Family");

		juniors.change_text(std::to_string(info.juniors) + "/" + std::to_string(info.max_juniors));
		reps.change_text(number(info.rep) + " / " + number(info.total_rep));
		today.change_text(info.today >= 0 ? "+" + number(info.today) : number(info.today));

		if (precepts.get_text() != info.precepts)
		{
			precepts.change_text(info.precepts);
			precepts_x = PRECEPTS_W;
		}

		int32_t self = Stage::get().get_player().get_oid();
		buttons[PRECEPTS]->set_state(family().is_leader(self) ? Button::State::NORMAL : Button::State::DISABLED);

		show_privilege();
		enable_buttons();
	}

	void UIFamily::show_privilege()
	{
		const std::vector<FamilyPrivilege>& privileges = family().get_privileges();

		if (selected >= privileges.size())
			selected = 0;

		if (privileges.empty())
			return;

		const FamilyPrivilege& privilege = privileges[selected];
		int32_t times = family().get_info().used_today(static_cast<int32_t>(selected));

		name.change_text(privilege.name);
		cost.change_text(number(privilege.cost));
		used.change_text(std::to_string(times) + " / " + std::to_string(privilege.limit));

		char counter[16];
		std::snprintf(counter, sizeof(counter), "[%02d/%02d]", static_cast<int>(selected + 1), static_cast<int>(privileges.size()));
		page.change_text(counter);

		description = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, privilege.description, DESCRIPTION_W);
	}

	void UIFamily::enable_buttons()
	{
		const FamilyInfo& info = family().get_info();

		auto enable = [this](Buttons id, bool on)
		{
			Button::State state = buttons[id]->get_state();

			if (!on)
				buttons[id]->set_state(Button::State::DISABLED);
			else if (state == Button::State::DISABLED)
				buttons[id]->set_state(Button::State::NORMAL);
		};

		enable(JUNIOR, info.juniors < info.max_juniors);
		enable(SPECIAL, family().can_use(selected));
	}

	void UIFamily::draw(float inter) const
	{
		background.draw(position);

		UIElement::draw(inter);

		Point<int16_t> raise(0, TEXT_RAISE);

		title.draw(position + Point<int16_t>(TITLE_X + TITLE_W / 2, TITLE_Y) - raise);
		juniors.draw(position + Point<int16_t>(JUNIORS_X + JUNIORS_W, JUNIORS_Y) - raise);
		reps.draw(position + Point<int16_t>(REPS_X + REPS_W, REPS_Y) - raise);
		today.draw(position + Point<int16_t>(TODAY_X + TODAY_W, TODAY_Y) - raise);

		if (!precepts.empty())
		{
			int16_t left = position.x() + PRECEPTS_X;

			precepts.draw_between(position + Point<int16_t>(PRECEPTS_X + precepts_x, PRECEPTS_Y) - raise,
				Range<int16_t>(left, left + PRECEPTS_W));
		}

		if (family().get_privileges().empty())
			return;

		name.draw(position + Point<int16_t>(NAME_X + NAME_W / 2, NAME_Y) - raise);
		cost.draw(position + Point<int16_t>(COST_X + COST_W, COST_Y) - raise);
		used.draw(position + Point<int16_t>(USED_X + USED_W, USED_Y) - raise);
		page.draw(position + Point<int16_t>(PAGE_X, PAGE_Y) - raise);
		description.draw(position + Point<int16_t>(DESCRIPTION_X, DESCRIPTION_Y) - raise);
	}

	void UIFamily::update()
	{
		UIElement::update();

		if (!precepts.empty() && ++precepts_step >= PRECEPTS_STEPS)
		{
			precepts_step = 0;
			precepts_x--;

			if (precepts_x + precepts.width() < 0)
				precepts_x = PRECEPTS_W;
		}

		enable_buttons();
	}

	Cursor::State UIFamily::send_cursor(bool clicked, Point<int16_t> cursorpos)
	{
		Cursor::State state = UIDragElement::send_cursor(clicked, cursorpos);

		if (dragged)
		{
			clear_tooltip();

			return state;
		}

		Point<int16_t> local = cursorpos - position;
		std::string text;

		if (inside(REPS_TIP, local))
		{
			text = REPS_HELP;
		}
		else if (inside(JUNIORS_TIP, local))
		{
			text = JUNIORS_HELP;
		}
		else if (inside(USED_TIP, local) && !family().get_privileges().empty())
		{
			const FamilyPrivilege& privilege = family().get_privileges()[selected];
			int32_t times = family().get_info().used_today(static_cast<int32_t>(selected));

			if (times >= privilege.limit)
				text = USED_UP;
			else
				text = "Entitlement used today: " + std::to_string(times)
					+ "\\nEntitlement available today: " + std::to_string(privilege.limit - times);
		}

		if (text.empty())
		{
			clear_tooltip();
		}
		else
		{
			UI::get().show_text(Tooltip::Parent::FAMILY, text);
			tooltip = true;
		}

		return state;
	}

	void UIFamily::remove_cursor()
	{
		UIDragElement::remove_cursor();

		clear_tooltip();
	}

	void UIFamily::clear_tooltip()
	{
		if (!tooltip)
			return;

		tooltip = false;
		UI::get().clear_tooltip(Tooltip::Parent::FAMILY);
	}

	void UIFamily::send_key(int32_t, bool pressed, bool escape)
	{
		if (pressed && escape)
		{
			clear_tooltip();
			deactivate();
		}
	}

	Button::State UIFamily::button_pressed(uint16_t buttonid)
	{
		size_t count = family().get_privileges().size();

		switch (buttonid)
		{
		case JUNIOR:
			FamilyActions::prompt_junior();
			break;
		case TREE:
			FamilyActions::open_tree(Stage::get().get_player().get_stats().get_name());
			break;
		case LEFT:
			if (count > 0)
				selected = (selected + count - 1) % count;

			show_privilege();
			break;
		case RIGHT:
			if (count > 0)
				selected = (selected + 1) % count;

			show_privilege();
			break;
		case PRECEPTS:
			edit_precepts();
			break;
		case SPECIAL:
			use_privilege();
			break;
		case OK:
			deactivate();
			break;
		}

		return Button::State::NORMAL;
	}

	void UIFamily::use_privilege()
	{
		const std::vector<FamilyPrivilege>& privileges = family().get_privileges();

		if (!family().loaded() || selected >= privileges.size())
			return;

		int32_t index = static_cast<int32_t>(selected);

		if (!privileges[selected].needs_target())
			return UseFamilyPacket(index).dispatch();

		UI::get().emplace<UIEnterText>(TARGET_PROMPT, [index](const std::string& target)
		{
			UseFamilyPacket(index, target).dispatch();
		}, NAME_LIMIT);
	}

	void UIFamily::edit_precepts()
	{
		if (!family().loaded())
			return;

		UI::get().emplace<UIEnterText>(PRECEPTS_PROMPT, [](const std::string& text)
		{
			ChangeFamilyMessagePacket(text).dispatch();
		}, Family::MAX_PRECEPTS);
	}

	UIElement::Type UIFamily::get_type() const
	{
		return TYPE;
	}
}
