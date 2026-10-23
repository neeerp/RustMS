#include "UIPersonalShop.h"

#include "UICharInfo.h"
#include "UIChatBar.h"
#include "UIItemInventory.h"
#include "UINpcTalk.h"
#include "UINotice.h"

#include "../UI.h"
#include "../Window.h"

#include "../Components/Charset.h"
#include "../Components/MapleButton.h"
#include "../../Audio/Audio.h"

#include "../../Character/ExclRequest.h"
#include "../../Character/Inventory/ItemProperties.h"
#include "../../Constants.h"
#include "../../Data/CurseData.h"
#include "../../Data/ItemData.h"
#include "../../Gameplay/Stage.h"
#include "../../Graphics/Geometry.h"
#include "../../Graphics/GraphicsGL.h"
#include "../../Net/Packets/MiniRoomPackets.h"
#include "../../Net/Packets/PlayerInteractionPackets.h"
#include "../../Util/Misc.h"
#include "../../Util/NxFiles.h"

#include <algorithm>
#include <nlnx/nx.hpp>

namespace ms
{
	namespace
	{
		using personal_shop::OWNER;
		using personal_shop::SEATS;

		// The v83 dialog's controls and drawing (FUN_006fa4b2, FUN_006fb99d).
		constexpr Point<int16_t> BT_TOP = Point<int16_t>(144, 62);
		constexpr Point<int16_t> BT_MIDDLE = Point<int16_t>(144, 90);
		constexpr Point<int16_t> BT_BOTTOM = Point<int16_t>(144, 110);
		constexpr Point<int16_t> BT_ENTER = Point<int16_t>(482, 321);
		constexpr Point<int16_t> BT_CLAIM = Point<int16_t>(498, 343);
		constexpr int16_t BT_BAN_Y = 117;

		// v83 draws a line's cap top about 3 below where it is drawn, we 9,
		// so all text here goes up by the difference.
		constexpr Point<int16_t> TEXT_Y = Point<int16_t>(0, -6);

		constexpr Point<int16_t> TITLE = Point<int16_t>(23, 9);
		constexpr size_t TITLE_WIDTH = 496;
		constexpr Point<int16_t> MESO = Point<int16_t>(218, 135);

		// The owner stands on the left facing right, the visitors to the
		// right of them facing left; names go on the tags below.
		constexpr Point<int16_t> OWNER_FEET = Point<int16_t>(53, 127);
		constexpr int16_t OWNER_NAME_X = 56;
		constexpr int16_t FEET_Y = 127;
		constexpr int16_t NAME_Y = 135;
		constexpr int16_t SEAT_PITCH = 95;

		int16_t seat_x(uint8_t slot)
		{
			return (slot + 2) * SEAT_PITCH;
		}

		// Five rows of items, 40 apart.
		constexpr int16_t ROW_TOP = 157;
		constexpr int16_t ROW_PITCH = 40;
		constexpr int16_t ROWS = 5;
		constexpr int16_t ROW_LEFT = 7;
		constexpr Point<int16_t> ROW_SIZE = Point<int16_t>(199, 35);
		constexpr Point<int16_t> SELECT = Point<int16_t>(44, 0);
		constexpr Point<int16_t> ICON = Point<int16_t>(8, 34);
		constexpr Point<int16_t> COUNT = Point<int16_t>(8, 23);
		constexpr Point<int16_t> NAME = Point<int16_t>(48, 1);
		constexpr Point<int16_t> PRICE = Point<int16_t>(48, 19);
		constexpr size_t NAME_WIDTH = 152;
		constexpr int16_t ITEMS_X = 211;
		const Range<int16_t> ITEMS_RANGE = Range<int16_t>(158, 355);

		// The chat log: nine rows of 16, wrapped to 243.
		constexpr int16_t LOG_LEFT = 241;
		constexpr int16_t LOG_TOP = 162;
		constexpr int16_t LOG_ROW = 16;
		constexpr int16_t LOG_ROWS = 9;
		constexpr int16_t LOG_WIDTH = 243;
		constexpr size_t LOG_LIMIT = 200;
		constexpr int16_t LOG_CLIP_TOP = 160;
		constexpr int16_t LOG_CLIP_BOTTOM = 321;
		constexpr int16_t CHAT_X = 502;
		const Range<int16_t> CHAT_RANGE = Range<int16_t>(157, 318);

		const Rectangle<int16_t> INPUT = Rectangle<int16_t>(238, 469, 323, 338);
		constexpr size_t INPUT_LIMIT = 70;

		// Clicks pass through the empty corner right of the fee table.
		constexpr Point<int16_t> CLEAR_CORNER = Point<int16_t>(231, 367);
		constexpr int16_t DRAG_HEIGHT = 156;

		// A line takes the colour of its speaker's name tag.
		constexpr Color::Name SEAT_COLORS[SEATS] = {
			Color::Name::BLACK, Color::Name::SEATBLUE, Color::Name::SEATPINK, Color::Name::DEEPGREEN
		};

		constexpr Color::Name PRICE_COLORS[] = {
			Color::Name::PRICEBLUE, Color::Name::PRICETEAL, Color::Name::PRICEGREEN,
			Color::Name::PRICEORANGE, Color::Name::PRICEPINK, Color::Name::PRICEPLUM
		};

		// A hired merchant's owner's buttons and countdown (0x6fa4b2, 0x518314).
		constexpr Point<int16_t> BT_ARRANGE = Point<int16_t>(144, 42);
		constexpr Point<int16_t> BT_COIN = Point<int16_t>(114, 135);
		constexpr Point<int16_t> BT_BLACKLIST = Point<int16_t>(236, 345);
		constexpr Point<int16_t> BT_VISITORS = Point<int16_t>(306, 345);
		constexpr Point<int16_t> COUNTDOWN = Point<int16_t>(380, 11);
		constexpr Point<int16_t> COUNTDOWN_BOX = Point<int16_t>(130, 12);

		const std::string FULL = "You can't sell it anymore.";
		const std::string TOO_PRICEY = "The total price for the items exceeded\\nthe maximum amount of mesos that can be sold.";

		Point<int16_t> centred(Point<int16_t> dimension)
		{
			return Point<int16_t>(
					(Constants::Constants::get().get_viewwidth() - dimension.x()) / 2,
					(Constants::Constants::get().get_viewheight() - dimension.y()) / 2
			);
		}

		void notice(const std::string &text)
		{
			UI::get().emplace<UIOk>(text, [](bool) {});
		}

		// The slot and id of the hired merchant permit the server was asked
		// about (0xa27d17).
		std::pair<int16_t, int32_t> hiring;
	}

	UIPersonalShop::UIPersonalShop(const mini_room::Enter &enter) : UIDragElement<PosPERSONALSHOP>(),
			own_slot(enter.own_slot < SEATS ? enter.own_slot : OWNER), merchant(enter.type == mini_room::HIRED_MERCHANT), merchant_meso(enter.meso), open_ms(enter.open_ms),
			entered(std::chrono::steady_clock::now()), max_items(enter.max_items), selected(-1),
			first_row(0), log_rows(0), log_row(0)
	{
		nl::node PersonalShop = NxFiles::ui_classic["UIWindow.img"]["PersonalShop"];
		nl::node Basic = NxFiles::ui_classic["Basic.img"];

		backgrnd = PersonalShop["backgrnd6"];
		select = PersonalShop["select"];

		// Visitors only ever see an open shop; a merchant's owner comes back
		// to one open unless it is still being set up (0x518b0a).
		open = !is_owner() || (merchant && !enter.setup);

		// Each side gets only its own buttons (FUN_006fc46b).
		if (is_owner())
		{
			buttons[Buttons::ITEM] = std::make_unique<MapleButton>(PersonalShop["BtItem"], BT_TOP);
			buttons[Buttons::CLOSE] = std::make_unique<MapleButton>(PersonalShop["BtClose"], BT_BOTTOM);

			// A merchant's owner in maintenance reopens it instead (0x518b0a).
			if (open)
				buttons[Buttons::RESUME] = std::make_unique<MapleButton>(PersonalShop["BtExit"], BT_MIDDLE);
			else
				buttons[Buttons::START] = std::make_unique<MapleButton>(PersonalShop["BtStart"], BT_MIDDLE);

			if (merchant)
			{
				nl::node EntrustedShop = NxFiles::ui_classic["UIWindow.img"]["EntrustedShop"];

				buttons[Buttons::ARRANGE] = std::make_unique<MapleButton>(EntrustedShop["BtArrange"], BT_ARRANGE);
				buttons[Buttons::COIN] = std::make_unique<MapleButton>(EntrustedShop["BtCoin"], BT_COIN);
			}

			// A ban button shows over each visitor while they stay, though
			// not in a merchant (0x518873).
			for (uint8_t slot = 1; slot < SEATS && !merchant; slot++)
			{
				uint16_t id = Buttons::BAN1 + slot - 1;
				buttons[id] = std::make_unique<MapleButton>(PersonalShop["BtBan"], Point<int16_t>(seat_x(slot) + 30, BT_BAN_Y));
				buttons[id]->set_active(false);
			}
		} else
		{
			// A merchant's visitors can't look its owner up (0x518873).
			if (!merchant)
				buttons[Buttons::INFO] = std::make_unique<MapleButton>(PersonalShop["BtInfo"], BT_TOP);

			buttons[Buttons::BUY] = std::make_unique<MapleButton>(PersonalShop["BtBuy"], BT_MIDDLE);
			buttons[Buttons::EXIT] = std::make_unique<MapleButton>(PersonalShop["BtExit"], BT_BOTTOM);
		}

		// A merchant's lists, which only its owner may ask for (0x518c50).
		if (merchant)
		{
			buttons[Buttons::BLACKLIST] = std::make_unique<MapleButton>(PersonalShop["BtBlackList"], BT_BLACKLIST);
			buttons[Buttons::VISITORS] = std::make_unique<MapleButton>(PersonalShop["BtVisit"], BT_VISITORS);

			if (!is_owner())
				for (uint16_t id : { Buttons::BLACKLIST, Buttons::VISITORS })
					buttons[id]->set_state(Button::State::DISABLED);
		}

		buttons[Buttons::ENTER] = std::make_unique<MapleButton>(PersonalShop["BtEnter"], BT_ENTER);
		buttons[Buttons::CLAIM] = std::make_unique<MapleButton>(Basic["BtClaim"], BT_CLAIM);

		// A merchant's caption names its owner (SP 0xd86).
		std::string caption = merchant ? enter.owner + "'s Hired Merchant : " + enter.title : enter.title;
		title = Text(Text::Font::A12B, Text::Alignment::LEFT, Color::Name::BLACK, caption);
		string_format::format_with_ellipsis(title, TITLE_WIDTH);

		meso = Text(Text::Font::A12M, Text::Alignment::RIGHT, Color::Name::BLACK);

		for (Member &member : members)
			member.name = Text(Text::Font::A12M, Text::Alignment::CENTER, Color::Name::WHITE);

		for (const mini_room::Member &member : enter.members)
			visit(member);

		if (merchant)
		{
			// Slot 0 is the merchant's employee (0x518531).
			employee = nl::nx::item["Cash"]["0503.img"][string_format::extend_id(enter.employee, 8)]["employee"]["stand"]["0"];
			countdown = Text(Text::Font::A12B, Text::Alignment::LEFT, Color::Name::BLACK);
			sales = hired_merchant::sales(enter.sold, enter.net);
		}

		items = Slider(
				Slider::Type::CLASSIC, ITEMS_RANGE, ITEMS_X, ROWS, 0,
				[&](bool upwards)
				{
					int16_t shift = upwards ? -1 : 1;

					if (first_row + shift >= 0 && first_row + shift + ROWS <= static_cast<int16_t>(listings.size()))
						first_row += shift;
				}
		);

		chat = Slider(
				Slider::Type::CLASSIC, CHAT_RANGE, CHAT_X, LOG_ROWS, 0,
				[&](bool upwards)
				{
					int16_t shift = upwards ? -1 : 1;

					if (log_row + shift >= 0 && log_row + shift + LOG_ROWS <= log_rows)
						log_row += shift;
				}
		);

		input = Textfield(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, INPUT, INPUT_LIMIT);

		input.set_enter_callback(
				[&](std::string text)
				{
					input.change_text("");
					submit(text);
				}
		);

		input.set_key_callback(
				KeyAction::Id::ESCAPE,
				[&]()
				{
					input.set_state(Textfield::State::DISABLED);
					input.set_state(Textfield::State::NORMAL);
				}
		);

		set_listings(enter.listings);

		// What was said before we came in (0x518888).
		for (const auto &[line, slot] : enter.chat)
			add_chat(slot, line);

		dimension = backgrnd.get_dimensions();
		dragarea = Point<int16_t>(dimension.x(), DRAG_HEIGHT);
	}

	void UIPersonalShop::draw(float inter) const
	{
		static const Charset countset = Charset(NxFiles::ui_classic["Basic.img"]["ItemNo"], Charset::Alignment::LEFT);

		backgrnd.draw(position);
		title.draw(position + TITLE + TEXT_Y);

		for (uint8_t slot = 0; slot < SEATS; slot++)
		{
			const Member &member = members[slot];

			if (!member.present)
				continue;

			// Our own avatar follows our equipment as it changes.
			const CharLook &look = slot == own_slot ? Stage::get().get_player().get_look() : member.look;

			if (slot == OWNER)
			{
				// The employee stands bottom centre on the spot, its origin
				// ignored (0x5181ee).
				if (merchant)
					employee.draw(position + OWNER_FEET + employee.get_origin()
						- Point<int16_t>(employee.get_dimensions().x() / 2, employee.get_dimensions().y()));
				else
					look.draw(position + OWNER_FEET, true, Stance::Id::STAND1, Expression::Id::DEFAULT);

				member.name.draw(position + Point<int16_t>(OWNER_NAME_X, NAME_Y) + TEXT_Y);
			} else
			{
				look.draw(position + Point<int16_t>(seat_x(slot), FEET_Y), false, Stance::Id::STAND1, Expression::Id::DEFAULT);
				member.name.draw(position + Point<int16_t>(seat_x(slot) - 2, NAME_Y) + TEXT_Y);
			}
		}

		meso.draw(position + MESO + TEXT_Y);

		// On a white strip, only while there is time left (0x518314).
		if (merchant && is_owner() && !countdown.get_text().empty())
		{
			ColorBox(COUNTDOWN_BOX.x(), COUNTDOWN_BOX.y(), Color::Name::WHITE, 1.0f).draw(position + COUNTDOWN);
			countdown.draw(position + COUNTDOWN + TEXT_Y);
		}

		for (int16_t k = 0; k < ROWS; k++)
		{
			auto index = static_cast<size_t>(first_row + k);

			if (index >= listings.size())
				break;

			const mini_room::Listing &listing = listings[index];
			int32_t itemid = listing.item.itemid;
			Point<int16_t> row = position + Point<int16_t>(0, ROW_TOP + ROW_PITCH * k);

			// A sold out row stays selected but loses its highlight.
			if (static_cast<int16_t>(index) == selected && listing.bundles != 0)
				select.draw(row + SELECT);

			ItemData::get(itemid).get_icon(false).draw(row + ICON);

			if (listing.bundles == 0)
				ColorBox(ROW_SIZE.x(), ROW_SIZE.y(), Color::Name::WHITE, 0.5f).draw(row + Point<int16_t>(ROW_LEFT, 0));

			if (personal_shop::shows_count(itemid))
				countset.draw(std::to_string(listing.bundles * listing.per_bundle), row + COUNT);

			rows[index].name.draw(row + NAME + TEXT_Y);
			rows[index].price.draw(row + PRICE + TEXT_Y);
		}

		items.draw(position);

		Range<int16_t> clip = Range<int16_t>(position.y() + LOG_CLIP_TOP, position.y() + LOG_CLIP_BOTTOM);
		int16_t y = LOG_TOP - log_row * LOG_ROW;

		for (const Line &line : log)
		{
			if (y + line.rows * LOG_ROW > LOG_TOP && y < LOG_TOP + LOG_ROWS * LOG_ROW)
				line.text.draw(DrawArgument(position + Point<int16_t>(LOG_LEFT, y) + TEXT_Y), clip);

			y += line.rows * LOG_ROW;
		}

		chat.draw(position);
		input.draw(position + TEXT_Y);

		UIElement::draw_buttons(inter);
	}

	void UIPersonalShop::update()
	{
		UIElement::update();

		// A merchant's owner sees what it holds (0x51819a).
		int64_t shown = merchant && is_owner() ? merchant_meso : Stage::get().get_player().get_inventory().get_meso();
		meso.change_text(mini_room::meso_text(shown));

		if (merchant && is_owner())
		{
			auto since = std::chrono::steady_clock::now() - entered;
			countdown.change_text(hired_merchant::countdown_text(open_ms + std::chrono::duration_cast<std::chrono::milliseconds>(since).count()));
		}

		input.update(position);
	}

	void UIPersonalShop::remove_cursor()
	{
		UIDragElement::remove_cursor();

		items.remove_cursor();
		chat.remove_cursor();
		clear_tooltip();
	}

	Cursor::State UIPersonalShop::send_cursor(bool clicked, Point<int16_t> cursorpos)
	{
		Cursor::State dstate = UIDragElement::send_cursor(clicked, cursorpos);

		if (dragged)
		{
			clear_tooltip();

			return dstate;
		}

		Point<int16_t> cursor = cursorpos - position;
		last_cursor = cursor;

		for (Slider *slider : { &items, &chat })
		{
			if (Cursor::State sstate = slider->send_cursor(cursor, clicked))
			{
				clear_tooltip();

				return sstate;
			}
		}

		Cursor::State tstate = input.send_cursor(cursorpos, clicked);

		if (tstate != Cursor::State::IDLE)
			return tstate;

		int16_t index = row_at(cursor);

		if (index < 0)
		{
			clear_tooltip();

			return dstate;
		}

		const mini_room::Listing &listing = listings[index];

		// Sold out rows still show what they were (FUN_006fb864).
		UI::get().show_item(Tooltip::Parent::PERSONALSHOP, listing.item);

		if (clicked)
		{
			selected = index;

			// Before opening, or in a merchant's maintenance, the owner can
			// take a row back by dragging it to the inventory (FUN_006fb368).
			if (is_owner() && (!open || merchant) && listing.bundles >= 1)
			{
				Point<int16_t> top_left = Point<int16_t>(ICON.x(), ROW_TOP + ROW_PITCH * (index - first_row) + ICON.y() - 32);
				const Texture &icon = ItemData::get(listing.item.itemid).get_icon(false);

				dragged_row = Icon(std::make_unique<RowIcon>(index, listing.item.itemid), icon, -1);
				dragged_row.start_drag(cursor - top_left);
				UI::get().drag_icon(&dragged_row);

				clear_tooltip();

				return Cursor::State::GRABBING;
			}
		}

		return dstate;
	}

	bool UIPersonalShop::send_icon(const Icon &icon, Point<int16_t> cursorpos)
	{
		// Only the owner's own inventory items go in; the item checks follow
		// in the icon (FUN_004f41b9).
		if (is_owner())
			icon.drop_on_shop();

		return true;
	}

	void UIPersonalShop::doubleclick(Point<int16_t> cursorpos)
	{
		int16_t index = row_at(cursorpos - position);

		if (index >= 0)
			buy(index);
	}

	void UIPersonalShop::send_scroll(double yoffset)
	{
		if (last_cursor.x() > ITEMS_X + 20)
			chat.send_scroll(yoffset);
		else
			items.send_scroll(yoffset);
	}

	void UIPersonalShop::send_key(int32_t, bool pressed, bool escape)
	{
		if (pressed && escape)
		{
			if (merchant)
				close_merchant(2);
			else
				exit();
		}
	}

	bool UIPersonalShop::is_in_range(Point<int16_t> cursorpos) const
	{
		Point<int16_t> at = cursorpos - position;

		if (at.x() > CLEAR_CORNER.x() && at.y() > CLEAR_CORNER.y())
			return false;

		return UIElement::is_in_range(cursorpos);
	}

	UIElement::Type UIPersonalShop::get_type() const
	{
		return TYPE;
	}

	void UIPersonalShop::visit(const mini_room::Member &member)
	{
		if (member.slot >= members.size())
			return;

		Member &seat = members[member.slot];
		seat.present = true;

		// A merchant's slot 0 is its employee, with no look.
		if (!merchant || member.slot != OWNER)
			seat.look = CharLook(member.look);
		seat.name.change_text(member.name);

		// A merchant has no ban buttons.
		if (member.slot != OWNER && is_owner() && !merchant)
			buttons[Buttons::BAN1 + member.slot - 1]->set_active(true);
	}

	void UIPersonalShop::seat_left(uint8_t slot)
	{
		if (slot == OWNER || slot >= members.size())
			return;

		members[slot].present = false;
		members[slot].name.change_text("");

		if (is_owner() && !merchant)
			buttons[Buttons::BAN1 + slot - 1]->set_active(false);
	}

	void UIPersonalShop::change_look(uint8_t slot, const LookEntry &look)
	{
		if (slot < members.size() && slot != own_slot && members[slot].present)
			members[slot].look = CharLook(look);
	}

	void UIPersonalShop::add_chat(uint8_t slot, const std::string &line)
	{
		Color::Name color = SEAT_COLORS[slot < SEATS ? slot : OWNER];

		add_line(slot == own_slot ? line : mini_room::filter_chat(line, CurseData::filter), color);
	}

	void UIPersonalShop::set_listings(const std::vector<mini_room::Listing> &list)
	{
		listings = list;

		refresh_rows();
	}

	void UIPersonalShop::sold(const personal_shop::SoldNotice &notice)
	{
		if (notice.index >= listings.size())
			return;

		sales.add(listings[notice.index], notice.bundles, notice.buyer);

		if (auto dialog = UI::get().get_element<UIShopSales>(); dialog && dialog->is_active())
			dialog->refresh();
	}

	void UIPersonalShop::buy_result(int8_t result)
	{
		std::string text = personal_shop::buy_result_text(result);

		if (!text.empty())
			notice(text);
	}

	bool UIPersonalShop::is_merchant() const
	{
		return merchant;
	}

	void UIPersonalShop::set_merchant_meso(int32_t meso)
	{
		merchant_meso = meso;
	}

	void UIPersonalShop::organized(int32_t meso)
	{
		merchant_meso = meso;

		listings.erase(
				std::remove_if(listings.begin(), listings.end(), [](const mini_room::Listing &listing) { return listing.bundles == 0; }),
				listings.end()
		);
		selected = -1;

		refresh_rows();
	}

	void UIPersonalShop::store_closed(int8_t code)
	{
		std::string text = hired_merchant::close_text(code);

		close_merchant(1);
		notice(text);
	}

	void UIPersonalShop::meso_taken()
	{
		merchant_meso = 0;
	}

	void UIPersonalShop::list_shown(UIElement::Type list, bool shown)
	{
		uint16_t id = list == UIElement::Type::MERCHANTBLACKLIST ? Buttons::BLACKLIST : Buttons::VISITORS;

		if (merchant && is_owner())
			buttons[id]->set_state(shown ? Button::State::DISABLED : Button::State::NORMAL);
	}

	void UIPersonalShop::leave(int8_t reason)
	{
		// A merchant's reasons, some of which keep the window (0x518d5f).
		if (merchant)
		{
			hired_merchant::Leave left = hired_merchant::leave(reason);

			if (left.closes)
				close_merchant(1);

			notice(left.text);

			return;
		}

		std::string text = personal_shop::leave_text(reason);

		close();

		if (!text.empty())
			notice(text);
	}

	void UIPersonalShop::close()
	{
		clear_tooltip();
		UI::get().cancel_drag();

		if (input.get_state() == Textfield::State::FOCUSED)
			input.set_state(Textfield::State::DISABLED);

		input.set_state(Textfield::State::NORMAL);
		input.change_text("");

		for (UIElement::Type type : { UIElement::Type::SHOPSELL, UIElement::Type::SHOPSALES, UIElement::Type::MERCHANTVISITORS,
				UIElement::Type::MERCHANTBLACKLIST })
			UI::get().remove(type);

		deactivate();
	}

	void UIPersonalShop::sell(InventoryType::Id tab, int16_t slot, int32_t itemid, int16_t count)
	{
		// Nothing more goes in once the shop is full or open, though a
		// merchant takes more in maintenance (FUN_006fd96c).
		if (listings.size() >= max_items || (open && !merchant))
		{
			notice(FULL);

			return;
		}

		if (auto dialog = UI::get().get_element<UIShopSell>(); dialog && dialog->is_active())
			return;

		// A merchant sells one of a one-of-a-kind item at a time (0x6fd9d6).
		if (merchant && ItemData::get(itemid).get_properties().only)
		{
			for (const mini_room::Listing &listing : listings)
			{
				if (listing.item.itemid == itemid)
				{
					notice("You may only sell one of this item.");

					return;
				}
			}
		}

		// A set of stars or bullets counts as one.
		int16_t quantity = shop_list::is_rechargeable(itemid) ? 1 : count;

		UI::get().emplace<UIShopSell>(tab, slot, itemid, quantity, merchant);
	}

	void UIPersonalShop::take_back(int16_t index) const
	{
		if (!is_owner() || (open && !merchant) || index < 0 || index >= static_cast<int16_t>(listings.size()))
			return;

		int32_t itemid = listings[index].item.itemid;

		// Nor can its owner hold two of a one-of-a-kind item.
		if (merchant && ItemData::get(itemid).get_properties().only
			&& Stage::get().get_player().get_inventory().get_total_item_count(itemid) > 0)
		{
			notice("You may only possess one of this item.");

			return;
		}

		// The row leaves when the server sends the new list.
		if (listings[index].bundles >= 1)
			Stage::get().get_player().request(ShopTakeBackPacket(index, merchant), ExclRequest::WAIT);
	}

	const personal_shop::Sales &UIPersonalShop::get_sales() const
	{
		return sales;
	}

	uint8_t UIPersonalShop::get_own_slot() const
	{
		return own_slot;
	}

	void UIPersonalShop::create(int16_t slot, int32_t itemid, bool merchant)
	{
		// Not on a mount, then not while transformed or a ghost (FUN_00a1dd99).
		Player &player = Stage::get().get_player();

		if (player.ride_refuses() || player.shape_refuses() || player.ghost_refuses())
			return;

		if (UI::get().dialog_open())
		{
			if (auto chatbar = UI::get().get_element<UIChatBar>())
				chatbar->send_chatline("Not available", UIChatBar::LineType::SKILL);

			return;
		}

		if (!Stage::get().get_mapinfo().allows_personal_shop())
		{
			notice("The personal store can only be built\\nat the free market available at main towns.");

			return;
		}

		UI::get().emplace<UIEnterText>(
				"Please enter the name of the store.\\n\\nIf you move before you open the store,\\n the store may not be opened.",
				[slot, itemid, merchant](const std::string &title)
				{
					if (title.empty())
						return;

					// v83 (FUN_00a1dd99) opens the store with curse words
					// starred out rather than refusing the name.
					auto send = [name = CurseData::filter(title), slot, itemid, merchant]()
					{
						ShopCreatePacket(name, slot, itemid, merchant).dispatch();
						Sound(nl::nx::sound["Game.img"]["UseShopItem"]).play();
					};

					if (!merchant)
					{
						send();

						return;
					}

					// Fredrick's contract comes first, and closing it any way
					// sets the merchant up (0xa1e0a5).
					UI::get().emplace<UINpcTalk>();
					UI::get().enable();

					if (auto talk = UI::get().get_element<UINpcTalk>())
					{
						talk->change_text(hired_merchant::FREDRICK, UINpcTalk::SENDOK, 0, 0, hired_merchant::CONTRACT);
						talk->set_local_responder([send](int8_t, int32_t) { send(); });
					}
				},
				50
		);
	}

	void UIPersonalShop::hire(int16_t slot, int32_t itemid, int64_t cash_id)
	{
		Player &player = Stage::get().get_player();

		if (!Stage::get().get_mapinfo().allows_personal_shop())
		{
			notice("The personal store can only be built\\nat the free market available at main towns.");

			return;
		}

		if (!player.get_phobj().onground)
		{
			if (auto chatbar = UI::get().get_element<UIChatBar>())
				chatbar->send_chatline("You may not open the store while jumping.", UIChatBar::LineType::SKILL);

			return;
		}

		// v83 also refuses near a portal (712be4); the server checks that.
		if (player.get_level() <= hired_merchant::MIN_LEVEL)
		{
			notice("Hired Merchant's Shop is only available for \\ncharacters that are over Level 15.");

			return;
		}

		hiring = { slot, itemid };

		EntrustedShopRequestPacket(cash_id).dispatch();
	}

	void UIPersonalShop::hire_approved()
	{
		create(hiring.first, hiring.second, true);
	}

	Button::State UIPersonalShop::button_pressed(uint16_t buttonid)
	{
		clear_tooltip();

		switch (buttonid)
		{
			case Buttons::START:
				// Opening an empty shop does nothing (FUN_006fcc19).
				if (!listings.empty() && !open)
				{
					ShopBlacklistPacket({}).dispatch();
					ShopOpenPacket().dispatch();

					// The owner leaves a merchant to it (0x51876c).
					if (merchant)
					{
						close_merchant(1);

						return Button::State::NORMAL;
					}

					open = true;
					buttons[Buttons::START]->set_state(Button::State::DISABLED);

					return Button::State::DISABLED;
				}

				break;
			case Buttons::BUY:
				buy(selected);
				break;
			case Buttons::CLOSE:
				if (merchant)
					close_merchant(8);
				else
					exit();

				return Button::State::NORMAL;
			case Buttons::EXIT:
				exit();

				return Button::State::NORMAL;
			case Buttons::RESUME:
				close_merchant(2);

				return Button::State::NORMAL;
			case Buttons::ARRANGE:
				// Only with something listed, once the notice is read (0x519294).
				if (!listings.empty())
					UI::get().emplace<UIOk>(
							"The sold items are crossed off the list, and\\nthe sales fee will be retrieved.",
							[](bool) { MerchantPacket(MerchantPacket::Request::ORGANIZE).dispatch(); }
					);

				break;
			case Buttons::COIN:
				// Only with mesos to take (0x51930f).
				if (merchant_meso != 0)
					MerchantPacket(MerchantPacket::Request::TAKE_MESO).dispatch();

				break;
			case Buttons::BLACKLIST:
				// The server answers with the list (0x519382).
				if (auto list = UI::get().get_element<UIMerchantBlacklist>(); !list || !list->is_active())
					MerchantPacket(MerchantPacket::Request::BLACKLIST).dispatch();

				break;
			case Buttons::VISITORS:
				if (auto list = UI::get().get_element<UIMerchantVisitors>(); !list || !list->is_active())
					MerchantPacket(MerchantPacket::Request::VISITORS).dispatch();

				break;
			case Buttons::INFO:
				show_owner_info();
				break;
			case Buttons::ITEM:
				if (auto dialog = UI::get().get_element<UIShopSales>(); !dialog || !dialog->is_active())
					UI::get().emplace<UIShopSales>();

				break;
			case Buttons::ENTER:
			{
				std::string text = input.get_text();
				input.change_text("");
				submit(text);
				break;
			}
			case Buttons::BAN1:
			case Buttons::BAN2:
			case Buttons::BAN3:
				ban(static_cast<uint8_t>(buttonid - Buttons::BAN1 + 1));
				break;
			default:
				// Like the status bar's, the claim button does nothing.
				break;
		}

		return Button::State::NORMAL;
	}

	bool UIPersonalShop::is_owner() const
	{
		return own_slot == OWNER;
	}

	int16_t UIPersonalShop::row_at(Point<int16_t> cursor) const
	{
		if (cursor.x() < ROW_LEFT || cursor.x() >= ROW_LEFT + ROW_SIZE.x() || cursor.y() < ROW_TOP)
			return -1;

		int16_t k = (cursor.y() - ROW_TOP) / ROW_PITCH;

		if (k >= ROWS || (cursor.y() - ROW_TOP) % ROW_PITCH >= ROW_SIZE.y())
			return -1;

		int16_t index = first_row + k;

		return index < static_cast<int16_t>(listings.size()) ? index : -1;
	}

	void UIPersonalShop::buy(int16_t index)
	{
		// Only visitors buy, from a row that exists (FUN_006fd261).
		if (is_owner() || index < 0 || index >= static_cast<int16_t>(listings.size()))
			return;

		const mini_room::Listing &listing = listings[index];
		auto row = static_cast<uint8_t>(index);

		if (listing.bundles <= 0)
		{
			notice("The item has been sold already.");

			return;
		}

		// v83 doesn't check our mesos or space; the server answers that.
		auto send = [row, merchant = merchant](int16_t bundles, int64_t total)
		{
			Player &player = Stage::get().get_player();

			if (personal_shop::over_low_level_limit(player.get_stats().get_stat(MapleStat::Id::LEVEL), total))
			{
				notice(personal_shop::buy_result_text(0xe));

				return;
			}

			player.request(ShopBuyPacket(row, bundles, merchant), ExclRequest::WAIT);
		};

		mini_room::Listing chosen = listing;

		auto order = [send, chosen]()
		{
			if (chosen.item.expiration < item_expiry::now())
			{
				notice("The time limit has expired.");

				return;
			}

			if (personal_shop::sells_whole(chosen.item.itemid, chosen.item.expiration))
			{
				int64_t total = chosen.price;

				UI::get().emplace<UIYesNo>(
						chosen.item.cash ? personal_shop::CASH_BUY_QUESTION : "Are you sure you want to buy it?",
						[send, total](bool yes)
						{
							if (yes)
								send(1, total);
						}
				);

				return;
			}

			UI::get().emplace<UIEnterNumber>(
					"How many are you willing to buy?",
					[send, chosen](int32_t units)
					{
						personal_shop::Purchase purchase = personal_shop::purchase(chosen, units);

						if (!purchase.problem.empty())
						{
							notice(purchase.problem);

							return;
						}

						UI::get().emplace<UIYesNo>(
								personal_shop::total_question(purchase.total),
								[send, purchase](bool yes)
								{
									if (yes)
										send(purchase.bundles, purchase.total);
								}
						);
					},
					chosen.bundles * chosen.per_bundle, 1
			);
		};

		// Buying uses up Scissors of Karma (FUN_006fd261).
		auto karma = [order, chosen]()
		{
			if (!chosen.item.cash && item_properties::karma(chosen.item.itemid, chosen.item.flags))
				UI::get().emplace<UIYesNo>("Once purchased, the item cannot be traded.\\nWould you still like to purchase it?",
					[order](bool yes)
					{
						if (yes)
							order();
					});
			else
				order();
		};

		// A cash item with a time limit asks first that it was checked.
		if (listing.item.cash && listing.item.expiration < item_expiry::BEFORE)
		{
			Text name = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK,
				ItemData::get(listing.item.itemid).get_name());
			string_format::format_with_ellipsis(name, 100);

			UI::get().emplace<UIYesNo>(
				mini_room::time_limit_line(name.get_text(), listing.item.expiration, item_expiry::now()) +
					mini_room::CHECK_TIME_LIMIT,
				[karma](bool yes)
				{
					if (yes)
						karma();
				});
		}
		else
		{
			karma();
		}
	}

	void UIPersonalShop::ban(uint8_t slot)
	{
		if (slot >= members.size() || !members[slot].present)
			return;

		std::string name = members[slot].name.get_text();

		UI::get().emplace<UIYesNo>(
				"Would you like to ban " + name + "?",
				[slot, name](bool yes)
				{
					if (yes)
						ShopBanPacket(slot, name).dispatch();
				}
		);
	}

	void UIPersonalShop::show_owner_info() const
	{
		// The owner's, not a visitor's (FUN_006fb07b).
		const std::string &name = members[OWNER].name.get_text();

		for (auto &mmo : *Stage::get().get_chars().get_chars())
		{
			auto character = static_cast<const Char *>(mmo.second.get());

			if (character->get_name() == name)
			{
				int32_t cid = character->get_oid();

				if (Stage::get().get_player().request(CharInfoRequestPacket(cid), ExclRequest::USE_WAIT))
					UI::get().emplace<UICharInfo>(cid);

				return;
			}
		}
	}

	void UIPersonalShop::submit(const std::string &text)
	{
		// The server adds our name and sends the line back.
		if (!text.empty())
			MiniRoomChatPacket(text).dispatch();
	}

	void UIPersonalShop::add_line(const std::string &line, Color::Name color)
	{
		bool at_bottom = log_row + LOG_ROWS >= log_rows;

		Text text = Text(Text::Font::A12M, Text::Alignment::LEFT, color, line, LOG_WIDTH, true, 2);
		auto rows = static_cast<int16_t>(std::max(1, (text.height() + LOG_ROW - 3) / LOG_ROW));

		log.push_back({ text, rows });
		log_rows += rows;

		if (log.size() > LOG_LIMIT)
		{
			log_rows -= log.front().rows;
			log_row = std::max<int16_t>(0, log_row - log.front().rows);
			log.erase(log.begin());
		}

		if (at_bottom)
			scroll_to_bottom();
		else
			chat.setrows(log_row, LOG_ROWS, log_rows);
	}

	void UIPersonalShop::scroll_to_bottom()
	{
		log_row = std::max<int16_t>(0, log_rows - LOG_ROWS);
		chat.setrows(log_row, LOG_ROWS, log_rows);
	}

	void UIPersonalShop::refresh_rows()
	{
		rows.clear();

		for (const mini_room::Listing &listing : listings)
		{
			Row row;
			row.name = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, ItemData::get(listing.item.itemid).get_name());
			string_format::format_with_ellipsis(row.name, NAME_WIDTH);

			row.price = Text(Text::Font::A12M, Text::Alignment::LEFT, PRICE_COLORS[personal_shop::price_tier(listing.price)],
					personal_shop::price_text(listing));

			rows.push_back(row);
		}

		auto count = static_cast<int16_t>(listings.size());

		first_row = std::clamp<int16_t>(first_row, 0, std::max<int16_t>(0, count - ROWS));
		items.setrows(first_row, ROWS, count);
	}

	void UIPersonalShop::clear_tooltip()
	{
		UI::get().clear_tooltip(Tooltip::Parent::PERSONALSHOP);
	}

	void UIPersonalShop::exit()
	{
		// No confirmation either way (FUN_006fb07b).
		MiniRoomLeavePacket().dispatch();

		close();
	}

	void UIPersonalShop::close_merchant(int8_t reason)
	{
		// Leaving one still being set up closes it.
		if (reason == 2 && !open)
			reason = 8;

		if (!is_owner() || (reason != 2 && reason != 8))
		{
			exit();

			return;
		}

		// The merchant reopens as the owner goes.
		if (reason == 2)
		{
			MerchantPacket(MerchantPacket::Request::REOPEN).dispatch();
			exit();

			return;
		}

		// What is left comes back through the server, which answers with
		// mode 0x2a; with nothing left, the owner just goes.
		auto finish = []()
		{
			auto shop = UI::get().get_element<UIPersonalShop>();

			if (!shop || !shop->is_active())
				return;

			if (!shop->listings.empty() || shop->merchant_meso != 0)
				MerchantPacket(MerchantPacket::Request::CLOSE).dispatch();
			else
				shop->exit();
		};

		if (!open)
		{
			finish();

			return;
		}

		UI::get().emplace<UIYesNo>(
				"If there's no slot available for the item at\\nthe inventory, the item can only be retrieved\\n"
				"through Fredrick, the Store Bank NPC.\\nWill you close the store?",
				[finish](bool yes)
				{
					if (yes)
						finish();
				}
		);
	}

	UIPersonalShop::RowIcon::RowIcon(int16_t index, int32_t itemid) : index(index), itemid(itemid)
	{}

	InventoryType::Id UIPersonalShop::RowIcon::get_return_tab() const
	{
		return InventoryType::by_item_id(itemid);
	}

	void UIPersonalShop::RowIcon::drop_on_inventory() const
	{
		if (auto shop = UI::get().get_element<UIPersonalShop>())
			shop->take_back(index);
	}

	namespace
	{
		// The sell dialog (FUN_006fe414).
		const Rectangle<int16_t> COUNT_BOX = Rectangle<int16_t>(118, 225, 32, 47);
		const Rectangle<int16_t> PRICE_BOX = Rectangle<int16_t>(118, 225, 52, 67);
		const Rectangle<int16_t> PER_BUNDLE_BOX = Rectangle<int16_t>(118, 225, 72, 87);
		constexpr Point<int16_t> CHECKBOX = Point<int16_t>(11, 92);
		constexpr Point<int16_t> CHECKBOX_SIZE = Point<int16_t>(90, 14);
		constexpr Point<int16_t> BAR = Point<int16_t>(6, 51);
		constexpr Point<int16_t> BT_OK = Point<int16_t>(131, 120);
		constexpr Point<int16_t> BT_CANCEL = Point<int16_t>(180, 120);
	}

	UIShopSell::UIShopSell(InventoryType::Id tab, int16_t slot, int32_t itemid, int16_t quantity, bool merchant) : UIElement(),
			tab(tab), slot(slot), itemid(itemid), quantity(quantity), merchant(merchant), whole(personal_shop::sells_whole(itemid)), bundled(false)
	{
		nl::node PersonalShop = NxFiles::ui_classic["UIWindow.img"]["PersonalShop"];
		nl::node Basic = NxFiles::ui_classic["Basic.img"];

		backgrnd = PersonalShop["backgrnd2"];
		bar = PersonalShop["bar"];
		checkboxes[0] = Basic["CheckBox"]["0"];
		checkboxes[1] = Basic["CheckBox"]["1"];

		buttons[Buttons::OK] = std::make_unique<MapleButton>(Basic["BtOK2"], BT_OK);
		buttons[Buttons::CANCEL] = std::make_unique<MapleButton>(Basic["BtCancel2"], BT_CANCEL);

		count = Textfield(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, COUNT_BOX, 4);
		price = Textfield(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, PRICE_BOX, 10);
		per_bundle = Textfield(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, PER_BUNDLE_BOX, 4);

		// A whole item sells one at a time; a stack offers all of it.
		count.change_text(std::to_string(whole ? 1 : quantity));
		price.change_text("0");
		per_bundle.change_text(std::to_string(whole ? quantity : 1));

		for (Textfield *field : { &count, &price, &per_bundle })
		{
			field->set_enter_callback([&](std::string) { confirm(); });
			field->set_key_callback(KeyAction::Id::ESCAPE, [&]() { cancel(); });
		}

		if (whole)
			count.set_state(Textfield::State::DISABLED);

		per_bundle.set_state(Textfield::State::DISABLED);
		(whole ? price : count).set_state(Textfield::State::FOCUSED);

		dimension = backgrnd.get_dimensions();
		position = centred(dimension);
	}

	void UIShopSell::draw(float inter) const
	{
		backgrnd.draw(position);

		// Checked, the price is per bundle.
		if (bundled)
			bar.draw(position + BAR);

		checkboxes[bundled].draw(DrawArgument(position + CHECKBOX + Point<int16_t>(0, 1), whole ? 0.5f : 1.0f));

		count.draw(position + TEXT_Y);
		price.draw(position + TEXT_Y);
		per_bundle.draw(position + TEXT_Y);

		UIElement::draw(inter);
	}

	void UIShopSell::update()
	{
		UIElement::update();

		count.update(position);
		price.update(position);
		per_bundle.update(position);
	}

	Cursor::State UIShopSell::send_cursor(bool clicked, Point<int16_t> cursorpos)
	{
		for (Textfield *field : { &count, &price, &per_bundle })
		{
			Cursor::State fstate = field->send_cursor(cursorpos, clicked);

			if (fstate != Cursor::State::IDLE)
				return fstate;
		}

		Rectangle<int16_t> checkbox = Rectangle<int16_t>(position + CHECKBOX, position + CHECKBOX + CHECKBOX_SIZE);

		if (!whole && checkbox.contains(cursorpos))
		{
			if (clicked)
			{
				bundled = !bundled;
				per_bundle.set_state(bundled ? Textfield::State::NORMAL : Textfield::State::DISABLED);

				return Cursor::State::IDLE;
			}

			return Cursor::State::CANCLICK;
		}

		return UIElement::send_cursor(clicked, cursorpos);
	}

	void UIShopSell::send_key(int32_t, bool pressed, bool escape)
	{
		if (pressed && escape)
			cancel();
	}

	UIElement::Type UIShopSell::get_type() const
	{
		return TYPE;
	}

	Button::State UIShopSell::button_pressed(uint16_t buttonid)
	{
		if (buttonid == Buttons::OK)
			confirm();
		else
			cancel();

		return Button::State::NORMAL;
	}

	void UIShopSell::confirm()
	{
		personal_shop::SellForm form = { count.get_text(), price.get_text(), per_bundle.get_text(), bundled };
		personal_shop::SellCheck check = personal_shop::check_sale(form, itemid, quantity);

		// A problem keeps the dialog open at the box to fix (FUN_006fe2c3).
		if (check.box != personal_shop::Box::NONE)
		{
			box(check.box).set_state(Textfield::State::FOCUSED);
			notice(check.problem);

			return;
		}

		cancel();

		const personal_shop::Sale &sale = check.sale;

		if (personal_shop::too_pricey(sale.bundles, sale.price))
		{
			notice(TOO_PRICEY);

			return;
		}


		Stage::get().get_player().request(
				ShopPutItemPacket(static_cast<int8_t>(tab), slot, sale.bundles, sale.per_bundle, sale.price, merchant),
				ExclRequest::WAIT
		);
	}

	void UIShopSell::cancel()
	{
		for (Textfield *field : { &count, &price, &per_bundle })
		{
			if (field->get_state() == Textfield::State::FOCUSED)
				field->set_state(Textfield::State::NORMAL);
		}

		deactivate();
	}

	Textfield &UIShopSell::box(personal_shop::Box which)
	{
		switch (which)
		{
			case personal_shop::Box::COUNT:
				return count;
			case personal_shop::Box::PER_BUNDLE:
				return per_bundle;
			default:
				return price;
		}
	}

	namespace
	{
		// The sold-items dialog (FUN_006fee6d, FUN_006ff133): four rows of
		// 58, the last one the totals.
		constexpr Point<int16_t> SALES_CLOSE = Point<int16_t>(213, 6);
		constexpr int16_t SALES_SLIDER_X = 212;
		const Range<int16_t> SALES_RANGE = Range<int16_t>(31, 260);
		constexpr int16_t SALES_ROWS = 4;
		constexpr int16_t SALES_PITCH = 58;
		constexpr Point<int16_t> SALES_ICON = Point<int16_t>(8, 75);
		constexpr Point<int16_t> SALES_NAME = Point<int16_t>(50, 32);
		constexpr Point<int16_t> SALES_SECOND = Point<int16_t>(50, 51);
		constexpr Point<int16_t> SALES_THIRD = Point<int16_t>(50, 70);
	}

	UIShopSales::UIShopSales() : UIElement(), first_row(0)
	{
		nl::node Basic = NxFiles::ui_classic["Basic.img"];

		backgrnd = NxFiles::ui_classic["UIWindow.img"]["PersonalShop"]["backgrnd3"];
		meso_icon = nl::nx::item["Special"]["0900.img"]["09000001"]["iconRaw"];

		buttons[Buttons::CLOSE] = std::make_unique<MapleButton>(Basic["BtClose"], SALES_CLOSE);

		totals = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, "Total Sales / Total Received");
		total = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK);
		received = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK);

		slider = Slider(
				Slider::Type::CLASSIC, SALES_RANGE, SALES_SLIDER_X, SALES_ROWS, 0,
				[&](bool upwards)
				{
					int16_t shift = upwards ? -1 : 1;

					if (first_row + shift >= 0 && first_row + shift + SALES_ROWS <= total_rows())
						first_row += shift;
				}
		);

		refresh();

		dimension = backgrnd.get_dimensions();
		position = centred(dimension);
	}

	void UIShopSales::draw(float inter) const
	{
		backgrnd.draw(position);

		for (int16_t k = 0; k < SALES_ROWS; k++)
		{
			auto index = static_cast<size_t>(first_row + k);
			Point<int16_t> row = position + Point<int16_t>(0, SALES_PITCH * k);

			if (index < records.size())
			{
				const Record &record = records[index];

				ItemData::get(record.itemid).get_icon(false).draw(row + SALES_ICON);
				record.name.draw(row + SALES_NAME + TEXT_Y);
				record.buyer.draw(row + SALES_SECOND + TEXT_Y);
				record.units.draw(row + SALES_THIRD + TEXT_Y);
			} else if (index == records.size())
			{
				meso_icon.draw(row + SALES_ICON);
				totals.draw(row + SALES_NAME + TEXT_Y);
				total.draw(row + SALES_SECOND + TEXT_Y);
				received.draw(row + SALES_THIRD + TEXT_Y);
			}
		}

		slider.draw(position);

		UIElement::draw(inter);
	}

	void UIShopSales::remove_cursor()
	{
		UIElement::remove_cursor();

		slider.remove_cursor();
	}

	Cursor::State UIShopSales::send_cursor(bool clicked, Point<int16_t> cursorpos)
	{
		if (Cursor::State sstate = slider.send_cursor(cursorpos - position, clicked))
			return sstate;

		return UIElement::send_cursor(clicked, cursorpos);
	}

	void UIShopSales::send_scroll(double yoffset)
	{
		slider.send_scroll(yoffset);
	}

	void UIShopSales::send_key(int32_t, bool pressed, bool escape)
	{
		if (pressed && escape)
			deactivate();
	}

	UIElement::Type UIShopSales::get_type() const
	{
		return TYPE;
	}

	void UIShopSales::refresh()
	{
		auto shop = UI::get().get_element<UIPersonalShop>();

		if (!shop)
			return;

		const personal_shop::Sales &sales = shop->get_sales();

		records.clear();

		for (const personal_shop::Sales::Sold &sold : sales.sold)
		{
			Record record;
			record.itemid = sold.itemid;
			record.name = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, ItemData::get(sold.itemid).get_name());
			string_format::format_with_ellipsis(record.name, NAME_WIDTH);
			record.buyer = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, "Buyer : " + sold.buyer);
			record.units = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK,
					"Number purchased : " + std::to_string(sold.units));

			records.push_back(record);
		}

		total.change_text(mini_room::meso_text(sales.total) + " mesos.");
		received.change_text(mini_room::meso_text(sales.received) + " mesos.");

		first_row = std::clamp<int16_t>(first_row, 0, static_cast<int16_t>(total_rows() - SALES_ROWS));
		slider.setrows(first_row, SALES_ROWS, total_rows());
	}

	Button::State UIShopSales::button_pressed(uint16_t)
	{
		deactivate();

		return Button::State::NORMAL;
	}

	int16_t UIShopSales::total_rows() const
	{
		// The totals row and one blank after it once the list scrolls
		// (FUN_006ffd6e).
		auto count = static_cast<int16_t>(records.size());

		return count < SALES_ROWS ? SALES_ROWS : count + 2;
	}

	namespace
	{
		// The owner's lists (0x51a1ec, 0x519781): rows of 18 from y 51, lit
		// behind when selected, and the buttons along the bottom.
		constexpr int16_t LIST_TOP = 51;
		constexpr int16_t LIST_PITCH = 18;
		constexpr Point<int16_t> LIST_TEXT = Point<int16_t>(3, 4);
		constexpr Color LIST_SELECTED = Color(0x244768FFu);

		constexpr int16_t VISITOR_X = 7;
		constexpr int16_t VISITOR_WIDTH = 189;
		constexpr int16_t VISIT_TIME_X = 125;
		constexpr Point<int16_t> BT_SAVE_NAME = Point<int16_t>(4, 245);
		constexpr Point<int16_t> BT_VISITORS_OK = Point<int16_t>(163, 245);

		constexpr int16_t BLACKLIST_LEFT_X = 7;
		constexpr int16_t BLACKLIST_RIGHT_X = 135;
		constexpr int16_t BLACKLIST_WIDTH = 124;
		constexpr Point<int16_t> BT_ADD = Point<int16_t>(5, 245);
		constexpr Point<int16_t> BT_DELETE = Point<int16_t>(42, 245);
		constexpr Point<int16_t> BT_BLACKLIST_OK = Point<int16_t>(227, 245);

		const std::string BLACKLIST_PROMPT = "Please enter the name of the character you'd like to add to the Blacklist.";
	}

	UIMerchantList::UIMerchantList(const std::string &background, int16_t x) : UIElement(), selected(-1), detail_x(x)
	{
		backgrnd = NxFiles::ui_classic["UIWindow.img"]["PersonalShop"][background];

		dimension = backgrnd.get_dimensions();
		position = centred(dimension);
	}

	void UIMerchantList::draw(float inter) const
	{
		backgrnd.draw(position);

		for (size_t index = 0; index < rows.size() && index < capacity(); index++)
		{
			Rectangle<int16_t> area = cell(index);
			Point<int16_t> origin = position + Point<int16_t>(area.left(), area.top());

			if (static_cast<int16_t>(index) == selected)
				GraphicsGL::get().drawrectangle(origin.x(), origin.y(), area.width(), area.height(),
						LIST_SELECTED.r(), LIST_SELECTED.g(), LIST_SELECTED.b(), LIST_SELECTED.a());

			rows[index].name_text.draw(origin + LIST_TEXT + TEXT_Y);
			rows[index].detail.draw(origin + Point<int16_t>(detail_x, LIST_TEXT.y()) + TEXT_Y);
		}

		UIElement::draw(inter);
	}

	Cursor::State UIMerchantList::send_cursor(bool clicked, Point<int16_t> cursorpos)
	{
		bool released = !clicked && mouse_down;
		mouse_down = clicked;

		Cursor::State state = UIElement::send_cursor(clicked, cursorpos);

		// The button coming up on a row selects it, and off the rows selects
		// none; the buttons take theirs first.
		if (released && !is_holding_button())
		{
			Point<int16_t> at = cursorpos - position;
			int16_t index = -1;

			for (size_t k = 0; k < capacity(); k++)
			{
				Rectangle<int16_t> area = cell(k);

				if (at.x() >= area.left() && at.x() < area.right() && at.y() >= area.top() && at.y() < area.bottom())
					index = static_cast<int16_t>(k);
			}

			select(index < static_cast<int16_t>(rows.size()) ? index : -1);
		}

		return state;
	}

	void UIMerchantList::remove_cursor()
	{
		UIElement::remove_cursor();

		mouse_down = false;
	}

	void UIMerchantList::send_key(int32_t keycode, bool pressed, bool escape)
	{
		if (pressed && (escape || keycode == KeyAction::Id::RETURN))
			close();
	}

	bool UIMerchantList::has_selection() const
	{
		return selected >= 0 && static_cast<size_t>(selected) < rows.size();
	}

	void UIMerchantList::close()
	{
		deactivate();

		if (auto shop = UI::get().get_element<UIPersonalShop>())
			shop->list_shown(get_type(), false);
	}

	void UIMerchantList::add_row(const std::string &name, const std::string &detail)
	{
		Row row;
		row.name = name;
		row.name_text = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, name);
		row.detail = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, detail);

		rows.push_back(row);
		select(selected);
	}

	void UIMerchantList::remove_selected()
	{
		if (!has_selection())
			return;

		rows.erase(rows.begin() + selected);

		// The selection stays where it was, on the next name if there is one
		// (0x519695).
		select(selected);
	}

	void UIMerchantList::select(int16_t index)
	{
		selected = index;

		for (size_t k = 0; k < rows.size(); k++)
		{
			Color::Name color = static_cast<int16_t>(k) == selected ? Color::Name::WHITE : Color::Name::BLACK;

			rows[k].name_text.change_color(color);
			rows[k].detail.change_color(color);
		}

		refresh_buttons();
	}

	UIMerchantVisitors::UIMerchantVisitors(const std::vector<hired_merchant::Visitor> &visitors)
			: UIMerchantList("backgrnd4", VISIT_TIME_X)
	{
		nl::node PersonalShop = NxFiles::ui_classic["UIWindow.img"]["PersonalShop"];

		buttons[Buttons::SAVE_NAME] = std::make_unique<MapleButton>(PersonalShop["BtSaveName"], BT_SAVE_NAME);
		buttons[Buttons::OK] = std::make_unique<MapleButton>(PersonalShop["BtOK"], BT_VISITORS_OK);

		for (const hired_merchant::Visitor &visitor : visitors)
			add_row(visitor.name, hired_merchant::visit_time_text(visitor.ms));

		refresh_buttons();
	}

	UIElement::Type UIMerchantVisitors::get_type() const
	{
		return TYPE;
	}

	Button::State UIMerchantVisitors::button_pressed(uint16_t buttonid)
	{
		// Save Name copies the name to the clipboard and tells no one.
		if (buttonid == Buttons::SAVE_NAME)
		{
			if (has_selection())
				Window::get().setclipboard(rows[selected].name);

			return Button::State::NORMAL;
		}

		close();

		return Button::State::NORMAL;
	}

	Rectangle<int16_t> UIMerchantVisitors::cell(size_t index) const
	{
		auto top = static_cast<int16_t>(LIST_TOP + LIST_PITCH * index);

		return Rectangle<int16_t>(VISITOR_X, VISITOR_X + VISITOR_WIDTH, top, top + LIST_PITCH);
	}

	size_t UIMerchantVisitors::capacity() const
	{
		return hired_merchant::VISITOR_ROWS;
	}

	void UIMerchantVisitors::refresh_buttons()
	{
		if (buttons.count(Buttons::SAVE_NAME))
			buttons[Buttons::SAVE_NAME]->set_state(has_selection() ? Button::State::NORMAL : Button::State::DISABLED);
	}

	UIMerchantBlacklist::UIMerchantBlacklist(const std::vector<std::string> &names) : UIMerchantList("backgrnd5", 0)
	{
		nl::node PersonalShop = NxFiles::ui_classic["UIWindow.img"]["PersonalShop"];

		buttons[Buttons::ADD] = std::make_unique<MapleButton>(PersonalShop["BtAdd"], BT_ADD);
		buttons[Buttons::REMOVE] = std::make_unique<MapleButton>(PersonalShop["BtDelete"], BT_DELETE);
		buttons[Buttons::OK] = std::make_unique<MapleButton>(PersonalShop["BtOK"], BT_BLACKLIST_OK);

		for (const std::string &name : names)
			add_row(name);

		refresh_buttons();
	}

	UIElement::Type UIMerchantBlacklist::get_type() const
	{
		return TYPE;
	}

	void UIMerchantBlacklist::add(const std::string &typed)
	{
		if (rows.size() >= capacity())
			return;

		std::string name = hired_merchant::trimmed(typed);
		std::vector<std::string> listed;

		for (const Row &row : rows)
			listed.push_back(row.name);

		std::string problem = hired_merchant::blacklist_problem(name, Stage::get().get_player().get_name(), listed);

		if (!problem.empty())
		{
			notice(problem);

			return;
		}

		add_row(name);
		MerchantBlacklistPacket(true, name).dispatch();
	}

	Button::State UIMerchantBlacklist::button_pressed(uint16_t buttonid)
	{
		switch (buttonid)
		{
			case Buttons::ADD:
				// Cancelling the prompt does nothing.
				UI::get().emplace<UIEnterText>(
						BLACKLIST_PROMPT,
						[](const std::string &text)
						{
							if (auto list = UI::get().get_element<UIMerchantBlacklist>(); list && list->is_active())
								list->add(text);
						},
						hired_merchant::BLACKLIST_NAME_MAX
				);
				break;
			case Buttons::REMOVE:
				if (has_selection())
				{
					MerchantBlacklistPacket(false, rows[selected].name).dispatch();
					remove_selected();
				}
				break;
			default:
				close();
				break;
		}

		return buttons[buttonid]->get_state() == Button::State::DISABLED ? Button::State::DISABLED : Button::State::NORMAL;
	}

	Rectangle<int16_t> UIMerchantBlacklist::cell(size_t index) const
	{
		int16_t left = index % 2 == 0 ? BLACKLIST_LEFT_X : BLACKLIST_RIGHT_X;
		auto top = static_cast<int16_t>(LIST_TOP + LIST_PITCH * (index / 2));

		return Rectangle<int16_t>(left, left + BLACKLIST_WIDTH, top, top + LIST_PITCH);
	}

	size_t UIMerchantBlacklist::capacity() const
	{
		return hired_merchant::BLACKLIST_MAX;
	}

	void UIMerchantBlacklist::refresh_buttons()
	{
		if (buttons.count(Buttons::ADD))
			buttons[Buttons::ADD]->set_state(rows.size() < capacity() ? Button::State::NORMAL : Button::State::DISABLED);

		if (buttons.count(Buttons::REMOVE))
			buttons[Buttons::REMOVE]->set_state(has_selection() ? Button::State::NORMAL : Button::State::DISABLED);
	}
}
