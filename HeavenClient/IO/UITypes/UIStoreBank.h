#pragma once

#include <cstdint>
#include <vector>

#include "../UIDragElement.h"

#include "../Components/Slider.h"
#include "../Graphics/Text.h"

#include "../Net/Handlers/StorageContents.h"

namespace ms
{
	// Fredrick's Store Bank (v83 CStoreBankDlg, 0x79a0ff): what a hired
	// merchant kept when it closed without its owner, its unsold items and
	// its mesos, all taken back at once for a storing fee.
	class UIStoreBank : public UIDragElement<PosSTOREBANK>
	{
	public:
		static constexpr Type TYPE = UIElement::Type::STOREBANK;
		static constexpr bool FOCUSED = true;
		static constexpr bool TOGGLED = false;

		UIStoreBank();

		void draw(float alpha) const override;

		void remove_cursor() override;

		Cursor::State send_cursor(bool clicked, Point<int16_t> position) override;

		void send_scroll(double yoffset) override;

		void send_key(int32_t keycode, bool pressed, bool escape) override;

		UIElement::Type get_type() const override;

		// Opens npcid's Store Bank with what it keeps.
		void open(int32_t npcid, const storage_contents::Contents &contents);

		// The server quoted the fee for Get All: ask before taking it all.
		void quote(int32_t days, int32_t fee);

		// The answer to a Get All confirmed.
		void answered(int8_t code);

		// A request came to nothing.
		void answered_nothing();

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

	private:
		struct Row
		{
			int32_t itemid;
			int16_t count;
			bool cash;
			Texture icon;
			Text name;
		};

		void refresh();

		void close();

		enum Buttons : uint16_t
		{
			EXIT,
			GET_ALL
		};

		int32_t npcid;
		Texture npc;
		Texture enabled_row;
		Texture cash_mark;
		Slider slider;
		Text meso_label;

		std::vector<Row> rows;
		int16_t offset;
		int32_t meso;

		// Get All is out, and waits for its answer (+0xbc).
		bool pending;
	};
}
