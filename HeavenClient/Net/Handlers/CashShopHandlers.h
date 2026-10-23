//////////////////////////////////////////////////////////////////////////////////
//	This file is part of the continued Journey MMORPG client					//
//	Copyright (C) 2015-2019  Daniel Allendorf, Ryan Payton						//
//																				//
//	This program is free software: you can redistribute it and/or modify		//
//	it under the terms of the GNU Affero General Public License as published by	//
//	the Free Software Foundation, either version 3 of the License, or			//
//	(at your option) any later version.											//
//																				//
//	This program is distributed in the hope that it will be useful,				//
//	but WITHOUT ANY WARRANTY; without even the implied warranty of				//
//	MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the				//
//	GNU Affero General Public License for more details.							//
//																				//
//	You should have received a copy of the GNU Affero General Public License	//
//	along with this program.  If not, see <https://www.gnu.org/licenses/>.		//
//////////////////////////////////////////////////////////////////////////////////
#pragma once

#include "../PacketHandler.h"

namespace ms
{
	// Handler for entering the Cash Shop
	class SetCashShopHandler : public PacketHandler
	{
	public:
		void handle(InPacket& recv) const override;

	private:
		void transition() const;
	};

	// Handler for the account's NX balances
	class QueryCashResultHandler : public PacketHandler
	{
	public:
		void handle(InPacket& recv) const override;
	};

	// Handler for the Cash Inventory, gifts and wish list
	class CashShopOperationHandler : public PacketHandler
	{
	public:
		void handle(InPacket& recv) const override;
	};

	// Handler for whether a name is free for a Name Change (0x148)
	class CheckNewNameResultHandler : public PacketHandler
	{
	public:
		void handle(InPacket& recv) const override;
	};

	// Handler for whether the character may ask for a Name Change (0x149)
	class NameChangeRulesResultHandler : public PacketHandler
	{
	public:
		void handle(InPacket& recv) const override;
	};

	// Handler for cancelling a Name Change with its receipt (0x71)
	class CancelNameChangeResultHandler : public PacketHandler
	{
	public:
		void handle(InPacket& recv) const override;
	};

	// Handler for a receipt that stands for no Name Change (0x78)
	class NoNameChangeHandler : public PacketHandler
	{
	public:
		void handle(InPacket& recv) const override;
	};
}