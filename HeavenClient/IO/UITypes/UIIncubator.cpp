#include "UIIncubator.h"

#include "UINotice.h"

#include "../UI.h"

#include "../Components/MapleButton.h"

#include "../../Constants.h"
#include "../../Character/Inventory/Incubator.h"
#include "../../Data/ItemData.h"
#include "../../Gameplay/Stage.h"
#include "../../Net/Packets/InventoryPackets.h"
#include "../../Util/NxFiles.h"

#include <nlnx/nx.hpp>

namespace ms
{
	namespace
	{
		// From the window's top left (0x81a999; the egg's 0x81aae6, whose operand order was not proven). The icon's
		// origin is its bottom left.
		constexpr Point<int16_t> ICON = Point<int16_t>(74, 118);
		// Where the egg's tooltip shows (0x81ad9a).
		constexpr Rectangle<int16_t> ICON_HOVER = Rectangle<int16_t>(75, 102, 100, 127);
		// As the Item Guard's: not checked against v83.
		constexpr int16_t DRAG_HEIGHT = 25;

		// StringPool 0x129.
		constexpr const char *REQUEST_BUSY = "Other requests are in process.\\nPlease try again in a little bit.";

		void notice(const std::string &text)
		{
			UI::get().emplace<UIOk>(text, [](bool) {});
		}
	}

	UIIncubator::UIIncubator(int16_t s, int32_t id, InventoryType::Id tab, int16_t es)
			: UIElement(Point<int16_t>(), Point<int16_t>()), slot(s), item_id(id),
			  egg_tab(InventoryType::Id::NONE), egg_slot(0), egg_id(0), dragged(false)
	{
		nl::node src = NxFiles::ui_classic["UIWindow.img"]["Incubator"];

		Texture background = src["backgrnd"];
		sprites.emplace_back(src["backgrnd"]);

		buttons[Buttons::OK] = std::make_unique<MapleButton>(src["BtOk"], Point<int16_t>(42, 154));
		buttons[Buttons::CANCEL] = std::make_unique<MapleButton>(src["BtCancel"], Point<int16_t>(100, 154));

		// Centred, as the Item Guard's.
		dimension = background.get_dimensions();
		position = Point<int16_t>((Constants::Constants::get().get_viewwidth() - dimension.x()) / 2,
								  (Constants::Constants::get().get_viewheight() - dimension.y()) / 2);

		// An egg double-clicked goes in as it is (0x4f01b5), the DS Egg too; the server refuses that one.
		if (tab != InventoryType::Id::NONE)
			put(tab, es);
	}

	void UIIncubator::draw(float alpha) const
	{
		UIElement::draw(alpha);

		if (int32_t id = egg())
			ItemData::get(id).get_icon(false).draw(position + ICON);
	}

	void UIIncubator::remove_cursor()
	{
		UIElement::remove_cursor();

		dragged = false;
		clear_tooltip();
	}

	Cursor::State UIIncubator::send_cursor(bool clicked, Point<int16_t> cursorpos)
	{
		if (dragged)
		{
			if (clicked)
			{
				position = cursorpos - drag_from;

				return Cursor::State::CLICKING;
			}

			dragged = false;
		}

		Point<int16_t> cursor = cursorpos - position;

		// 0x81ad9a: whatever is in the egg's place, while the cursor is over it.
		const Inventory &inventory = Stage::get().get_player().get_inventory();
		int32_t id = egg();

		if (id && ICON_HOVER.contains(cursor))
		{
			if (auto item = inventory.get_item(egg_tab, egg_slot))
				UI::get().show_item(Tooltip::Parent::INCUBATOR, id, item->get_expiration(), item->get_flags(), true, 0,
									item->get_owner());
		}
		else
		{
			clear_tooltip();
		}

		Cursor::State ret = UIElement::send_cursor(clicked, cursorpos);

		if (clicked && cursor.y() >= 0 && cursor.y() < DRAG_HEIGHT && !is_holding_button())
		{
			drag_from = cursor;
			dragged = true;
		}

		return ret;
	}

	void UIIncubator::send_key(int32_t keycode, bool pressed, bool escape)
	{
		// Ids 1 and 2 (0x81ab54): closed, nothing sent.
		if (pressed && (escape || keycode == KeyAction::Id::RETURN))
			close();
	}

	bool UIIncubator::send_icon(const Icon &icon, Point<int16_t>)
	{
		// 0x4f46d2: an egg from the Item window, else a notice. A refused drop keeps the last.
		auto [tab, s] = icon.get_source();

		if (tab == InventoryType::Id::NONE || tab == InventoryType::Id::EQUIPPED)
			return true;

		if (!incubator::is_egg(Stage::get().get_player().get_inventory().get_item_id(tab, s)))
		{
			notice(incubator::NOT_AN_EGG);

			return true;
		}

		put(tab, s);

		return true;
	}

	UIElement::Type UIIncubator::get_type() const
	{
		return TYPE;
	}

	Button::State UIIncubator::button_pressed(uint16_t buttonid)
	{
		if (buttonid == Buttons::OK)
			ok();
		else
			close();

		return Button::State::NORMAL;
	}

	int32_t UIIncubator::egg() const
	{
		if (egg_tab == InventoryType::Id::NONE)
			return 0;

		int32_t id = Stage::get().get_player().get_inventory().get_item_id(egg_tab, egg_slot);

		return id == egg_id ? id : 0;
	}

	void UIIncubator::put(InventoryType::Id tab, int16_t s)
	{
		egg_tab = tab;
		egg_slot = s;
		egg_id = Stage::get().get_player().get_inventory().get_item_id(tab, s);
	}

	void UIIncubator::ok()
	{
		// 0x81ab54: a notice without an egg, else the question. The window stays while it is asked.
		if (!egg())
		{
			notice(incubator::SELECT);

			return;
		}

		UI::get().emplace<UIYesNo>(incubator::ASK, [](bool yes)
		{
			if (!yes)
				return;

			if (auto window = UI::get().get_element<UIIncubator>())
				window->incubate();
		});
	}

	void UIIncubator::incubate()
	{
		// HeavenClient also checks the egg and the incubator are still in their places.
		Player &player = Stage::get().get_player();

		if (!egg() || player.get_inventory().get_item_id(InventoryType::Id::CASH, slot) != item_id)
			return;

		// A busy client says so and keeps the window open.
		if (!player.can_request(ExclRequest::WAIT))
		{
			notice(REQUEST_BUSY);

			return;
		}

		// No sound, and no chat line: INCUBATOR_RESULT says what hatched.
		if (player.request(UseOnItemPacket(slot, item_id, egg_tab, egg_slot), ExclRequest::WAIT))
			close();
	}

	void UIIncubator::close()
	{
		clear_tooltip();
		deactivate();
	}

	void UIIncubator::clear_tooltip()
	{
		UI::get().clear_tooltip(Tooltip::Parent::INCUBATOR);
	}
}
