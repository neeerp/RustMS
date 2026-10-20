#include "UISkillMacro.h"

#include "SkillMacroLayout.h"
#include "UINotice.h"

#include "../UI.h"

#include "../Components/MapleButton.h"

#include "../../Character/SkillMacros.h"
#include "../../Data/CurseData.h"
#include "../../Data/SkillData.h"
#include "../../Gameplay/SkillMacroRules.h"
#include "../../Gameplay/Stage.h"
#include "../../Net/Packets/PlayerPackets.h"
#include "../../Util/NxFiles.h"

#include <nlnx/nx.hpp>

namespace ms
{
	namespace
	{
		using namespace skill_macro_layout;

		// From the window's top left (FUN_008a8504 and FUN_008a8926). v83
		// copies a picture to its top left, whatever its origin.
		constexpr Point<int16_t> DOCK = Point<int16_t>(173, 0);
		constexpr Point<int16_t> HIGHLIGHT = Point<int16_t>(15, 51);
		constexpr Point<int16_t> SLOT = Point<int16_t>(17, 53);
		constexpr Point<int16_t> ICON_SLOT = Point<int16_t>(143, 53);
		constexpr Point<int16_t> TOP_LINE = Point<int16_t>(17, 49);
		constexpr Point<int16_t> BOTTOM_LINE = Point<int16_t>(17, 188);
		constexpr Point<int16_t> NAME_PANEL = Point<int16_t>(17, 195);
		constexpr Point<int16_t> SKILL = Point<int16_t>(SKILL_X, SKILL_Y);
		constexpr Point<int16_t> MACRO_ICON = Point<int16_t>(MACRO_ICON_X, SKILL_Y);
		constexpr Point<int16_t> SELECTED_ICON = Point<int16_t>(22, 200);
		// The scroll bar, 125 high with its arrows.
		constexpr int16_t SLIDER_X = 185;
		constexpr Range<int16_t> SLIDER = Range<int16_t>(54, 166);
		constexpr Point<int16_t> OK = Point<int16_t>(154, 265);
		// v83 draws a line's cap top about 3 below where it is drawn, we 9,
		// so text goes up by the difference.
		constexpr int16_t TEXT_RAISE = 6;
		constexpr Rectangle<int16_t> FIELD = Rectangle<int16_t>(59, 176, 215, 231);
		// The checkbox control; its box is drawn 2 right and 1 down in it.
		constexpr Rectangle<int16_t> CHECK = Rectangle<int16_t>(166, 181, 241, 256);
		constexpr Point<int16_t> CHECK_BOX = Point<int16_t>(2, 1);

		// StringPool 0x10f6, 0x110 and 0xce0.
		constexpr const char *EMPTY_SLOT = "Drag the skill icon from the skill inventory\\r\\non the left into the slots.";
		constexpr const char *NO_CURSING = "Cursing is not allowed.";
		constexpr const char *SAVED = "It is saved.";

		void send_macros()
		{
			SkillMacroPacket(UI::get().get_skill_macros()).dispatch();
		}

		bool may_hold(int32_t skill_id)
		{
			return skill_macro::may_hold(skill_id, SkillData::get(skill_id).has_cooltime());
		}

		Point<int16_t> row_offset(size_t row)
		{
			return Point<int16_t>(0, ROW_PITCH * static_cast<int16_t>(row));
		}

		Point<int16_t> column_offset(size_t column)
		{
			return Point<int16_t>(COLUMN_PITCH * static_cast<int16_t>(column), 0);
		}
	}

	UISkillMacro::SlotIcon::SlotIcon(int32_t id) : skill_id(id) {}

	void UISkillMacro::SlotIcon::drop_on_stage() const
	{
		if (auto window = UI::get().get_element<UISkillMacro>())
			window->drop_skill(skill_id, true, true, {});
	}

	Icon::IconType UISkillMacro::SlotIcon::get_type()
	{
		return Icon::IconType::MACRO_SKILL;
	}

	UISkillMacro::MacroIcon::MacroIcon(size_t i) : index(i) {}

	Icon::IconType UISkillMacro::MacroIcon::get_type()
	{
		return Icon::IconType::MACRO;
	}

	UISkillMacro::UISkillMacro(Point<int16_t> skillbook)
			: UIElement(skillbook + DOCK, Point<int16_t>()), scroll(0), selected(-1), shout(false), held(false),
			  checking(false), drag_index(0), drag_slot(0)
	{
		nl::node basic = NxFiles::ui_classic["Basic.img"];
		nl::node src = NxFiles::ui_classic["UIWindow.img"]["SkillMacro"];

		Texture background = src["backgrnd"];
		sprites.emplace_back(src["backgrnd"]);

		highlight = src["macroslot2"];
		slot = src["macroslot"];
		icon_slot = src["macroslot1"];
		top_line = src["line01"];
		bottom_line = src["line02"];
		name_panel = src["macroname"];

		for (size_t i = 0; i < macro_icons.size(); i++)
			macro_icons[i] = src["Macroicon"][std::to_string(i)]["icon"];

		checkbox[false] = basic["CheckBox"]["0"];
		checkbox[true] = basic["CheckBox"]["1"];

		buttons[Buttons::BT_OK] = std::make_unique<MapleButton>(basic["BtOK2"], OK);
		buttons[Buttons::BT_OK]->set_state(Button::State::DISABLED);

		slider = Slider(
				Slider::Type::CLASSIC, SLIDER, SLIDER_X, ROWS, SkillMacros::MAX_MACROS,
				[&](bool upwards)
				{
					int16_t next = static_cast<int16_t>(scroll) + (upwards ? -1 : 1);

					if (next >= 0 && next + ROWS <= SkillMacros::MAX_MACROS)
						scroll = next;
				}
		);

		field = Textfield(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, FIELD,
						  SkillMacros::MAX_NAME_BYTES);

		dimension = background.get_dimensions();
	}

	void UISkillMacro::draw(float alpha) const
	{
		UIElement::draw_sprites(alpha);

		auto copy = [&](const Texture &texture, Point<int16_t> at)
		{
			texture.draw(position + at + texture.get_origin());
		};

		const SkillMacros &macros = UI::get().get_skill_macros();

		if (selected >= 0 && static_cast<size_t>(selected) >= scroll && static_cast<size_t>(selected) < scroll + ROWS)
			copy(highlight, HIGHLIGHT + row_offset(selected - scroll));

		for (size_t k = 0; k < ROWS; k++)
		{
			copy(slot, SLOT + row_offset(k));
			copy(icon_slot, ICON_SLOT + row_offset(k));
		}

		copy(top_line, TOP_LINE);
		copy(bottom_line, BOTTOM_LINE);
		copy(name_panel, NAME_PANEL);

		for (size_t k = 0; k < ROWS; k++)
		{
			size_t index = scroll + k;

			for (size_t j = 0; j < 3; j++)
				if (int32_t skill_id = macros.skill(index, j))
					copy(SkillData::get(skill_id).get_icon(SkillData::Icon::NORMAL), SKILL + column_offset(j) + row_offset(k));

			// Every row has its icon, made or not.
			copy(macro_icons[index], MACRO_ICON + row_offset(k));
		}

		if (selected >= 0)
			copy(macro_icons[selected], SELECTED_ICON);

		slider.draw(position);
		field.draw(position - Point<int16_t>(0, TEXT_RAISE));
		checkbox[shout].draw(position + CHECK.get_left_top() + CHECK_BOX);

		UIElement::draw_buttons(alpha);
	}

	void UISkillMacro::update()
	{
		UIElement::update();

		field.update(position);
	}

	void UISkillMacro::toggle_active()
	{
		if (active)
		{
			release();
			clear_tooltip();
		}

		UIElement::toggle_active();
	}

	void UISkillMacro::remove_cursor()
	{
		UIElement::remove_cursor();

		slider.remove_cursor();

		// The button came up outside the window.
		if (checking)
		{
			checking = false;
			shout = !shout;
		}

		held = false;
		clear_tooltip();
	}

	Cursor::State UISkillMacro::send_cursor(bool clicked, Point<int16_t> cursorpos)
	{
		// v83 answers the button going down (FUN_008a98fe). The checkbox
		// toggles when it comes up, wherever it is, after going down on it.
		bool press = clicked && !held;
		bool released = !clicked && held;
		held = clicked;

		if (released && checking)
		{
			checking = false;
			shout = !shout;
		}

		Point<int16_t> cursor = cursorpos - position;

		if (Cursor::State state = slider.send_cursor(cursor, clicked))
		{
			clear_tooltip();

			return state;
		}

		if (Cursor::State state = field.send_cursor(cursorpos, clicked); state != Cursor::State::IDLE)
		{
			clear_tooltip();

			return state;
		}

		if (CHECK.contains(cursor))
		{
			clear_tooltip();

			if (press)
				checking = true;

			return clicked ? Cursor::State::CLICKING : Cursor::State::CANCLICK;
		}

		if (buttons[Buttons::BT_OK]->bounds(position).contains(cursorpos))
		{
			clear_tooltip();

			return UIElement::send_cursor(clicked, cursorpos);
		}

		Hit hit = hit_test(cursor.x(), cursor.y(), scroll, true);

		if (press)
		{
			clear_tooltip();
			select(hit.row);

			if (!hit.hit)
				return Cursor::State::CLICKING;

			Point<int16_t> row = row_offset(hit.row - scroll);

			if (hit.column == ICON_COLUMN)
			{
				start_drag(Icon(std::make_unique<MacroIcon>(hit.row), macro_icons[hit.row], -1),
						   cursor - MACRO_ICON - row);

				return Cursor::State::GRABBING;
			}

			if (int32_t skill_id = UI::get().get_skill_macros().skill(hit.row, hit.column))
			{
				drag_index = hit.row;
				drag_slot = hit.column;

				start_drag(Icon(std::make_unique<SlotIcon>(skill_id),
								SkillData::get(skill_id).get_icon(SkillData::Icon::NORMAL), -1),
						   cursor - SKILL - column_offset(hit.column) - row);

				return Cursor::State::GRABBING;
			}

			return Cursor::State::CLICKING;
		}

		if (hit.hit)
			show_tooltip(hit);
		else
			clear_tooltip();

		return UIElement::send_cursor(clicked, cursorpos);
	}

	void UISkillMacro::send_scroll(double yoffset)
	{
		clear_tooltip();
		slider.send_scroll(yoffset);
	}

	bool UISkillMacro::send_icon(const Icon &icon, Point<int16_t> cursorpos)
	{
		// v83 FUN_004faa22: only skills from the skill window or from a slot.
		switch (icon.get_type())
		{
			case Icon::IconType::SKILL:
				drop_skill(icon.get_mapping().action, false, false, cursorpos - position);
				break;
			case Icon::IconType::MACRO_SKILL:
				if (&icon == &dragged)
					drop_skill(UI::get().get_skill_macros().skill(drag_index, drag_slot), true, false,
							   cursorpos - position);
				break;
			default:
				break;
		}

		return true;
	}

	UIElement::Type UISkillMacro::get_type() const
	{
		return TYPE;
	}

	void UISkillMacro::dock(Point<int16_t> skillbook)
	{
		position = skillbook + DOCK;
	}

	void UISkillMacro::close()
	{
		release();
		clear_tooltip();

		scroll = 0;
		slider.setrows(ROWS, SkillMacros::MAX_MACROS);
		selected = -1;
		shout = false;
		checking = false;
		field.change_text("");
		buttons[Buttons::BT_OK]->set_state(Button::State::DISABLED);

		deactivate();
	}

	Button::State UISkillMacro::button_pressed(uint16_t buttonid)
	{
		if (buttonid == Buttons::BT_OK)
			save();

		return Button::State::NORMAL;
	}

	void UISkillMacro::select(size_t index)
	{
		const SkillMacros &macros = UI::get().get_skill_macros();

		selected = static_cast<int16_t>(index);
		field.change_text(macros.name(index));
		shout = macros.shouts(index);
		buttons[Buttons::BT_OK]->set_state(Button::State::NORMAL);
	}

	void UISkillMacro::start_drag(Icon icon, Point<int16_t> offset)
	{
		dragged = std::move(icon);
		dragged.start_drag(offset);

		UI::get().drag_icon(&dragged);
	}

	void UISkillMacro::drop_skill(int32_t skill_id, bool from_slot, bool on_field, Point<int16_t> cursor)
	{
		// FUN_004faa22 drops no skill the character hasn't learned.
		if (Stage::get().get_player().get_skills().get_level(skill_id) < 1 || !may_hold(skill_id))
			return;

		SkillMacros &macros = UI::get().get_skill_macros();

		// A skill moved between slots leaves its slot, and the list is sent,
		// before v83 looks for the slot it went to.
		if (from_slot && !on_field)
		{
			macros.clear_skill(drag_index, drag_slot);
			send_macros();
		}

		Hit hit{};

		if (!on_field)
		{
			hit = hit_test(cursor.x(), cursor.y(), scroll, false);

			if (!hit.hit)
				return;
		}

		if (from_slot)
			macros.clear_skill(drag_index, drag_slot);

		if (!on_field)
			macros.set_skill(hit.row, hit.column, skill_id);

		send_macros();
	}

	void UISkillMacro::save()
	{
		if (selected < 0)
			return;

		std::string name = field.get_text();

		// FUN_008aa134
		if (CurseData::filter(name) != name)
		{
			notice(NO_CURSING);

			return;
		}

		if (UI::get().get_skill_macros().set_name(selected, name, shout))
			send_macros();

		notice(SAVED);
	}

	void UISkillMacro::notice(const char *text)
	{
		if (field.get_state() == Textfield::State::FOCUSED)
			UI::get().remove_textfield();

		UI::get().emplace<UIOk>(text, [](bool) {});
	}

	void UISkillMacro::show_tooltip(const Hit &hit)
	{
		if (hit.column == ICON_COLUMN)
		{
			clear_tooltip();

			return;
		}

		int32_t skill_id = UI::get().get_skill_macros().skill(hit.row, hit.column);

		if (!skill_id)
		{
			UI::get().show_text(Tooltip::Parent::SKILLMACRO, EMPTY_SLOT);

			return;
		}

		const SkillBook &skills = Stage::get().get_player().get_skills();

		UI::get().show_skill(Tooltip::Parent::SKILLMACRO, skill_id, skills.get_level(skill_id),
							 skills.get_masterlevel(skill_id), skills.get_expiration(skill_id));
	}

	void UISkillMacro::clear_tooltip()
	{
		UI::get().clear_tooltip(Tooltip::Parent::SKILLMACRO);
	}

	void UISkillMacro::release()
	{
		if (dragged.get_drag())
			UI::get().cancel_drag();

		if (field.get_state() == Textfield::State::FOCUSED)
			UI::get().remove_textfield();
	}
}
