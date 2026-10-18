#include "UIItemMegaphone.h"

#include "UINotice.h"

#include "../UI.h"

#include "../Components/MapleButton.h"

#include "../../Constants.h"
#include "../../Data/ItemData.h"
#include "../../Gameplay/Stage.h"
#include "../../Net/Packets/InventoryPackets.h"
#include "../../Util/Megaphones.h"
#include "../../Util/NxFiles.h"

#include <nlnx/nx.hpp>

namespace ms
{
	namespace
	{
		// From the window's top left (FUN_005eb5b8). v83 draws a line's cap
		// top about 3 below where it is drawn, we 9, so text goes up by the
		// difference.
		constexpr int16_t TEXT_RAISE = 6;
		constexpr Rectangle<int16_t> FIELD = Rectangle<int16_t>(19, 221, 129, 145);
		// The item's well, and the bar under it with its name.
		constexpr Rectangle<int16_t> WELL = Rectangle<int16_t>(105, 137, 65, 97);
		constexpr Point<int16_t> NAME = Point<int16_t>(120, 101);
		// The checkbox control; its box is drawn 2 right and 1 down in it.
		constexpr Rectangle<int16_t> CHECK = Rectangle<int16_t>(14, 174, 161, 175);
		constexpr Point<int16_t> CHECK_BOX = Point<int16_t>(2, 1);

		// StringPool 0x129 and 0x1152.
		constexpr const char *REQUEST_BUSY = "Other requests are in process.\\nPlease try again in a little bit.";
		constexpr const char *NO_ITEM = "You have not selected an item.\\nAre you sure you still want to use the item?";
	}

	UIItemMegaphone::UIItemMegaphone(int16_t s, int32_t id)
			: UIElement(Point<int16_t>(), Point<int16_t>()), slot(s), item_id(id), item_tab(InventoryType::Id::NONE),
			  item_slot(0), shown_id(0), ear(true), held(false)
	{
		nl::node basic = NxFiles::ui_classic["Basic.img"];
		nl::node src = NxFiles::ui_classic["UIWindow.img"]["ItemMegaphone"];

		Texture background = src["backgrnd"];
		sprites.emplace_back(src["backgrnd"]);

		checkbox[false] = basic["CheckBox"]["0"];
		checkbox[true] = basic["CheckBox"]["1"];

		buttons[Buttons::OK] = std::make_unique<MapleButton>(basic["BtOK2"], Point<int16_t>(145, 186));
		buttons[Buttons::CANCEL] = std::make_unique<MapleButton>(basic["BtCancel2"], Point<int16_t>(186, 186));

		item_name = Text(Text::Font::A12M, Text::Alignment::CENTER, Color::Name::WHITE);

		field = Textfield(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, FIELD, Megaphones::MAX_LINE);
		field.set_enter_callback(
				[&](std::string)
				{
					send();
				}
		);
		field.set_key_callback(
				KeyAction::Id::ESCAPE,
				[&]()
				{
					close();
				}
		);

		// Centred, with no title bar to drag it by.
		dimension = background.get_dimensions();
		position = Point<int16_t>((Constants::Constants::get().get_viewwidth() - dimension.x()) / 2,
								  (Constants::Constants::get().get_viewheight() - dimension.y()) / 2);

		field.set_state(Textfield::State::FOCUSED);
	}

	void UIItemMegaphone::draw(float alpha) const
	{
		UIElement::draw(alpha);

		if (shown_id)
		{
			ItemData::get(shown_id).get_icon(false).draw(position + WELL.get_left_top() + Point<int16_t>(0, 32));
			item_name.draw(position + NAME - Point<int16_t>(0, TEXT_RAISE));
		}

		field.draw(position - Point<int16_t>(0, TEXT_RAISE));
		checkbox[ear].draw(position + CHECK.get_left_top() + CHECK_BOX);
	}

	void UIItemMegaphone::update()
	{
		UIElement::update();

		field.update(position);
	}

	void UIItemMegaphone::remove_cursor()
	{
		UIElement::remove_cursor();

		held = false;
		clear_tooltip();
	}

	Cursor::State UIItemMegaphone::send_cursor(bool clicked, Point<int16_t> cursorpos)
	{
		// The well and the checkbox answer only to the button going down.
		bool press = clicked && !held;
		held = clicked;

		Point<int16_t> cursor = cursorpos - position;

		Cursor::State state = field.send_cursor(cursorpos, clicked);

		if (state != Cursor::State::IDLE)
		{
			clear_tooltip();

			return state;
		}

		// FUN_005ebbe7 and FUN_005ebc20: a click takes the item back, and
		// the item shows its tooltip.
		if (shown_id && WELL.contains(cursor))
		{
			if (press)
			{
				set_item(InventoryType::Id::NONE, 0);
				clear_tooltip();

				return Cursor::State::CLICKING;
			}

			if (item_tab == InventoryType::Id::EQUIP)
				UI::get().show_equip(Tooltip::Parent::ITEMMEGAPHONE, item_slot);
			else
				UI::get().show_item(Tooltip::Parent::ITEMMEGAPHONE, shown_id);

			return Cursor::State::IDLE;
		}

		clear_tooltip();

		if (CHECK.contains(cursor))
		{
			if (press)
				ear = !ear;

			return clicked ? Cursor::State::CLICKING : Cursor::State::CANCLICK;
		}

		return UIElement::send_cursor(clicked, cursorpos);
	}

	void UIItemMegaphone::send_key(int32_t keycode, bool pressed, bool escape)
	{
		if (!pressed)
			return;

		// FUN_005ebd0f: Enter is OK, and Escape is Cancel.
		if (escape)
			close();
		else if (keycode == KeyAction::Id::RETURN)
			send();
	}

	bool UIItemMegaphone::send_icon(const Icon &icon, Point<int16_t> cursorpos)
	{
		icon.drop_on_megaphone();

		return true;
	}

	UIElement::Type UIItemMegaphone::get_type() const
	{
		return TYPE;
	}

	void UIItemMegaphone::set_item(InventoryType::Id tab, int16_t s)
	{
		int32_t id = tab == InventoryType::Id::NONE ? 0 : Stage::get().get_player().get_inventory().get_item_id(tab, s);

		item_tab = id ? tab : InventoryType::Id::NONE;
		item_slot = id ? s : 0;
		shown_id = id;
		item_name.change_text(id ? ItemData::get(id).get_name() : "");
	}

	Button::State UIItemMegaphone::button_pressed(uint16_t buttonid)
	{
		if (buttonid == Buttons::OK)
			send();
		else
			close();

		return Button::State::NORMAL;
	}

	void UIItemMegaphone::send()
	{
		// FUN_005ec4b7: nothing typed keeps the window open.
		if (field.empty())
			return;

		if (!shown_id)
		{
			ask(NO_ITEM, [](UIItemMegaphone &window)
			{
				window.send_checked();
			});

			return;
		}

		send_checked();
	}

	void UIItemMegaphone::send_checked()
	{
		Player &player = Stage::get().get_player();

		if (!player.can_request(ExclRequest::WAIT))
		{
			notice(REQUEST_BUSY);

			return;
		}

		UseItemMegaphonePacket packet(slot, item_id, field.get_text(), ear, item_tab, item_slot);

		if (player.request(std::move(packet), ExclRequest::WAIT))
			Sound(nl::nx::sound["Game.img"]["UseShopItem"]).play();

		close();
	}

	void UIItemMegaphone::close()
	{
		release_field();
		clear_tooltip();
		deactivate();
	}

	void UIItemMegaphone::notice(const std::string &text)
	{
		bool refocus = release_field();

		UI::get().emplace<UIOk>(
				text,
				[refocus](bool)
				{
					after_notice(refocus);
				}
		);
	}

	void UIItemMegaphone::ask(const std::string &question, std::function<void(UIItemMegaphone &)> on_yes)
	{
		bool refocus = release_field();

		UI::get().emplace<UIYesNo>(
				question,
				[refocus, on_yes](bool yes)
				{
					if (auto window = after_notice(refocus); window && yes)
						on_yes(*window);
				}
		);
	}

	Optional<UIItemMegaphone> UIItemMegaphone::after_notice(bool refocus)
	{
		auto window = UI::get().get_element<UIItemMegaphone>();

		if (!window || !window->is_active())
			return {};

		if (refocus)
			window->field.set_state(Textfield::State::FOCUSED);

		return window;
	}

	bool UIItemMegaphone::release_field()
	{
		if (field.get_state() != Textfield::State::FOCUSED)
			return false;

		UI::get().remove_textfield();

		return true;
	}

	void UIItemMegaphone::clear_tooltip()
	{
		UI::get().clear_tooltip(Tooltip::Parent::ITEMMEGAPHONE);
	}
}
